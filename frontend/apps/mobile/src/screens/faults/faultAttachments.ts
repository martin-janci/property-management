/**
 * Fault-report photo upload — the presigned direct-to-S3 attachment pipeline.
 *
 * Background: `ReportFaultScreen` lets the user attach up to five photos to a
 * fault report, but for a long time those photos were only ever held in local
 * state and silently dropped on submit. PR #2989 turned the silent drop into a
 * user-visible "photos not attached" warning; this module (GitHub #2999) is the
 * real fix — it actually uploads them.
 *
 * We reuse the same three-step presigned pattern the rest of the codebase
 * already uses for direct-to-storage uploads (documents' `POST
 * /api/v1/documents/upload-url` + `messaging`'s upload-url → PUT → link flow):
 *
 *   1. Ask the api-server for a short-lived presigned S3 PUT URL
 *      (`POST /api/v1/documents/upload-url`). The response carries the `url`
 *      to PUT to and the tenant-scoped `file_key` the object will live at.
 *   2. Stream the image bytes straight to S3 with that presigned URL
 *      (`FileSystem.uploadAsync`, BINARY_CONTENT). The `Content-Type` header
 *      MUST match the type the URL was signed for, or S3 rejects the PUT.
 *   3. Register the uploaded object against the fault by echoing the `file_key`
 *      back to `POST /api/v1/faults/{id}/attachments` (stored as `storage_url`).
 *
 * The api-server never proxies the bytes — step 2 goes phone → S3 directly.
 *
 * Each photo is uploaded independently: a single failure doesn't abort the rest
 * (the caller reports "N of M uploaded"), so a flaky connection that drops one
 * photo still attaches the others rather than losing them all.
 */

import * as FileSystem from 'expo-file-system/legacy';
import { apiRequest } from '../../hooks/useApi';
import { transcodeToJpeg } from '../../utils/imageCompression';

/** Response from `POST /api/v1/documents/upload-url` (mirrors `CreateUploadUrlResponse`). */
interface CreateUploadUrlResponse {
  /** Presigned S3 PUT URL — the client PUTs bytes here directly. */
  url: string;
  /** Storage key the object lives at; echoed back when registering the attachment. */
  file_key: string;
  /** MIME type the URL was signed for; MUST be sent as the PUT `Content-Type`. */
  content_type: string;
  method: string;
  expires_at?: string;
}

/** Outcome of uploading a batch of photos for one fault report. */
export interface PhotoUploadOutcome {
  /** Number of photos we attempted to upload. */
  total: number;
  /** Number that were uploaded AND registered against the fault. */
  uploaded: number;
  /** Number that failed at any step (presign / PUT / register). */
  failed: number;
}

/** Local-file metadata derived for a picked photo. */
interface PhotoMeta {
  /** The (possibly transcoded) local file URI to upload. */
  uri: string;
  /** Filename echoed as `original_filename`. */
  fileName: string;
  /** MIME type — sent to presign + PUT + attachment register. */
  mimeType: string;
}

/**
 * Map a lower-cased file extension to a backend-allowed image MIME type.
 * HEIC/HEIF are deliberately absent — those are transcoded to JPEG upstream in
 * {@link inferPhotoMeta} because the backend allow-list rejects them.
 */
const EXTENSION_MIME: Record<string, string> = {
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  png: 'image/png',
  webp: 'image/webp',
  gif: 'image/gif',
};

/** Extract the lower-cased extension from a local file URI, or '' if none. */
function extensionOf(uri: string): string {
  // Strip any query/fragment an emulator URI might carry, then take the tail.
  const clean = uri.split(/[?#]/)[0];
  const dot = clean.lastIndexOf('.');
  if (dot < 0) return '';
  return clean.slice(dot + 1).toLowerCase();
}

/**
 * Resolve the upload metadata for a picked photo URI.
 *
 * iOS gallery captures default to HEIC/HEIF, which the backend upload allow-list
 * rejects. Mirror `DocumentUploadScreen`/`ReportFaultScreen`'s existing recipe
 * and transcode those to JPEG before uploading; a transcode failure falls back
 * to the original bytes so the per-photo upload can still be attempted (and
 * fail loudly) rather than being dropped here.
 */
export async function inferPhotoMeta(uri: string): Promise<PhotoMeta> {
  const extension = extensionOf(uri);
  const isHeic = extension === 'heic' || extension === 'heif';

  if (isHeic) {
    try {
      const jpegUri = await transcodeToJpeg(uri);
      return {
        uri: jpegUri,
        fileName: `photo_${Date.now()}.jpg`,
        mimeType: 'image/jpeg',
      };
    } catch {
      // Transcode failed — fall through and upload the original bytes. The
      // backend allow-list will reject it, counting this photo as failed.
    }
  }

  const mimeType = EXTENSION_MIME[extension] ?? 'image/jpeg';
  const baseName = uri.split(/[?#]/)[0].split('/').pop();
  const fileName = baseName && baseName.includes('.') ? baseName : `photo_${Date.now()}.jpg`;
  return { uri, fileName, mimeType };
}

/** Best-effort size of a local file in bytes; 0 when it can't be read. */
async function fileSizeBytes(uri: string): Promise<number> {
  try {
    const info = await FileSystem.getInfoAsync(uri);
    return info.exists ? info.size : 0;
  } catch {
    return 0;
  }
}

/**
 * Run the full presign → PUT → register pipeline for a single photo.
 * Throws on any failed step so {@link uploadFaultPhotos} can count it.
 */
async function uploadOnePhoto(faultId: string, uri: string): Promise<void> {
  const meta = await inferPhotoMeta(uri);
  const sizeBytes = await fileSizeBytes(meta.uri);

  // Step 1 — presigned URL. Reuses the shared documents upload-url endpoint,
  // which mints a tenant-scoped storage key usable by any attachment flow.
  const presigned = await apiRequest<CreateUploadUrlResponse>('/api/v1/documents/upload-url', {
    method: 'POST',
    body: {
      file_name: meta.fileName,
      mime_type: meta.mimeType,
      size_bytes: sizeBytes,
    },
  });

  // Step 2 — PUT the raw bytes directly to S3. No auth/tenant headers here (the
  // URL is already signed); the Content-Type MUST match the signature.
  const putResult = await FileSystem.uploadAsync(presigned.url, meta.uri, {
    httpMethod: 'PUT',
    uploadType: FileSystem.FileSystemUploadType.BINARY_CONTENT,
    headers: { 'Content-Type': presigned.content_type },
  });
  if (putResult.status < 200 || putResult.status >= 300) {
    throw new Error(`S3 PUT failed with HTTP ${putResult.status}`);
  }

  // Step 3 — register the uploaded object against the fault. `storage_url`
  // carries the key S3 stored the object under (matches the messaging file_key
  // echo convention and the AddAttachmentRequest contract).
  await apiRequest(`/api/v1/faults/${faultId}/attachments`, {
    method: 'POST',
    body: {
      filename: presigned.file_key,
      original_filename: meta.fileName,
      content_type: presigned.content_type,
      size_bytes: sizeBytes,
      storage_url: presigned.file_key,
    },
  });
}

/**
 * Upload every picked photo for a freshly-created fault and register each as an
 * attachment. Photos are uploaded independently so one failure never drops the
 * rest; the returned {@link PhotoUploadOutcome} lets the caller tell the user
 * exactly how many made it.
 */
export async function uploadFaultPhotos(
  faultId: string,
  uris: string[]
): Promise<PhotoUploadOutcome> {
  const outcome: PhotoUploadOutcome = { total: uris.length, uploaded: 0, failed: 0 };

  // Sequential rather than parallel: bounded memory (each PUT streams a
  // multi-megapixel file) and gentler on a weak connection, matching the
  // conservative concurrency the compression pass already uses.
  for (const uri of uris) {
    try {
      await uploadOnePhoto(faultId, uri);
      outcome.uploaded += 1;
    } catch {
      outcome.failed += 1;
    }
  }

  return outcome;
}
