/**
 * Meter-reading photo upload — presigned direct-to-S3 pipeline
 * (code-review: mobile-rn-meter-photo-not-uploaded).
 *
 * Background: `MeterReadingScreen` lets the resident attach a photo of the
 * meter dial, but the submit handler only ever queued the *local* `file://`
 * URI in the reading's JSON body (under a `photoUri` key the api-server doesn't
 * even read). The image bytes were never uploaded anywhere — the photo was
 * silently dropped on submit. This module actually uploads it and returns the
 * tenant-scoped storage key, which the caller sends as the reading's
 * `photo_url` (the field `SubmitReading` on the api-server expects).
 *
 * It reuses the same presign → PUT recipe fault attachments already use
 * (`screens/faults/faultAttachments.ts`) and the shared documents upload-url
 * endpoint (`POST /api/v1/documents/upload-url`). The non-trivial HEIC→JPEG
 * transcode + MIME inference is reused directly via `inferPhotoMeta`.
 *
 *   1. Ask the api-server for a short-lived presigned S3 PUT URL. The response
 *      carries the `url` to PUT to and the tenant-scoped `file_key`.
 *   2. Stream the image bytes straight to S3 with that presigned URL. The
 *      `Content-Type` MUST match what the URL was signed for or S3 rejects it.
 *
 * Unlike a fault attachment there is NO separate "register" step: the reading
 * submission (`POST /api/v1/meters/readings`) carries the returned key directly
 * in `SubmitReading.photo_url`, so this returns the key rather than attaching it.
 */

import * as FileSystem from 'expo-file-system/legacy';
import { apiRequest } from '../../hooks/useApi';
import { inferPhotoMeta } from '../faults/faultAttachments';

/** Response from `POST /api/v1/documents/upload-url` (mirrors `CreateUploadUrlResponse`). */
interface CreateUploadUrlResponse {
  /** Presigned S3 PUT URL — the client PUTs bytes here directly. */
  url: string;
  /** Storage key the object lives at; returned to store as the reading's `photo_url`. */
  file_key: string;
  /** MIME type the URL was signed for; MUST be sent as the PUT `Content-Type`. */
  content_type: string;
  method: string;
  expires_at?: string;
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
 * Upload a single captured meter photo to storage via the presigned-URL
 * pipeline and return the tenant-scoped storage key to persist as the reading's
 * `photo_url`.
 *
 * Throws if the presign request or the S3 PUT fails, so the caller can decide
 * whether to submit the reading without the photo (and warn the user) or treat
 * the whole submit as failed — never silently dropping it as before.
 */
export async function uploadMeterPhoto(uri: string): Promise<string> {
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

  return presigned.file_key;
}
