/**
 * faultAttachments — presigned fault-photo upload pipeline (issue #2999).
 *
 * Regression guard for the follow-up to PR #2989: fault-report photos used to
 * be picked, compressed, and then silently dropped — this suite pins the three
 * network steps that now actually upload them:
 *
 *   1. request a presigned URL   → POST /api/v1/documents/upload-url
 *   2. PUT the bytes to storage  → FileSystem.uploadAsync (BINARY_CONTENT, PUT)
 *   3. attach the storage key    → POST /api/v1/faults/{id}/attachments
 *
 * apiRequest, expo-file-system/legacy, and the HEIC transcoder are mocked so the
 * pipeline is exercised without a device filesystem or a live api-server.
 */

// ─── Mocks (declared before importing the module under test) ───────────────

jest.mock('../../hooks/useApi', () => ({
  apiRequest: jest.fn(),
}));

const mockUploadAsync = jest.fn();
const mockGetInfoAsync = jest.fn();
jest.mock('expo-file-system/legacy', () => ({
  getInfoAsync: (...args: unknown[]) => mockGetInfoAsync(...args),
  uploadAsync: (...args: unknown[]) => mockUploadAsync(...args),
  FileSystemUploadType: { BINARY_CONTENT: 0, MULTIPART: 1 },
}));

const mockTranscodeToJpeg = jest.fn();
jest.mock('../../utils/imageCompression', () => ({
  transcodeToJpeg: (...args: unknown[]) => mockTranscodeToJpeg(...args),
}));

import { apiRequest } from '../../hooks/useApi';
import { inferPhotoMeta, uploadFaultPhotos } from './faultAttachments';

const mockApiRequest = apiRequest as jest.Mock;

const FAULT_ID = 'fault-42';

/** Wire apiRequest to answer the upload-url POST then the attachment POST. */
function primeHappyPath(fileKey = 'org-1/2026/09/abc_leak.jpg') {
  mockApiRequest.mockImplementation((path: string) => {
    if (path === '/api/v1/documents/upload-url') {
      return Promise.resolve({
        url: 'https://s3.example/put?sig=xyz',
        file_key: fileKey,
        content_type: 'image/jpeg',
        method: 'PUT',
        expires_at: 'soon',
      });
    }
    if (path.endsWith('/attachments')) {
      return Promise.resolve({ id: 'att-1' });
    }
    throw new Error(`unexpected path ${path}`);
  });
}

beforeEach(() => {
  jest.clearAllMocks();
  mockGetInfoAsync.mockResolvedValue({ exists: true, size: 12345 });
  mockUploadAsync.mockResolvedValue({ status: 200, body: '' });
});

describe('uploadFaultPhotos', () => {
  it('runs presign → PUT → attach for each photo and reports success', async () => {
    primeHappyPath();

    const outcome = await uploadFaultPhotos(FAULT_ID, ['file:///leak.jpg']);

    expect(outcome).toEqual({ total: 1, uploaded: 1, failed: 0 });

    // Step 1 — presigned URL request carries filename, mime and the real size.
    expect(mockApiRequest).toHaveBeenCalledWith('/api/v1/documents/upload-url', {
      method: 'POST',
      body: { file_name: 'leak.jpg', mime_type: 'image/jpeg', size_bytes: 12345 },
    });

    // Step 2 — bytes PUT straight to the presigned URL with the signed
    // Content-Type and no auth headers.
    expect(mockUploadAsync).toHaveBeenCalledWith(
      'https://s3.example/put?sig=xyz',
      'file:///leak.jpg',
      {
        httpMethod: 'PUT',
        uploadType: 0,
        headers: { 'Content-Type': 'image/jpeg' },
      }
    );

    // Step 3 — the returned file_key is echoed back as storage_url.
    expect(mockApiRequest).toHaveBeenCalledWith(`/api/v1/faults/${FAULT_ID}/attachments`, {
      method: 'POST',
      body: {
        filename: 'org-1/2026/09/abc_leak.jpg',
        original_filename: 'leak.jpg',
        content_type: 'image/jpeg',
        size_bytes: 12345,
        storage_url: 'org-1/2026/09/abc_leak.jpg',
      },
    });
  });

  it('does not register the attachment when the S3 PUT fails', async () => {
    primeHappyPath();
    mockUploadAsync.mockResolvedValue({ status: 403, body: 'AccessDenied' });

    const outcome = await uploadFaultPhotos(FAULT_ID, ['file:///leak.jpg']);

    expect(outcome).toEqual({ total: 1, uploaded: 0, failed: 1 });
    // upload-url was requested, but the attachment register must NOT fire.
    expect(mockApiRequest).toHaveBeenCalledWith('/api/v1/documents/upload-url', expect.anything());
    expect(mockApiRequest).not.toHaveBeenCalledWith(
      `/api/v1/faults/${FAULT_ID}/attachments`,
      expect.anything()
    );
  });

  it('counts a photo as failed when the presign request rejects', async () => {
    mockApiRequest.mockRejectedValue(new Error('HTTP 503'));

    const outcome = await uploadFaultPhotos(FAULT_ID, ['file:///leak.jpg']);

    expect(outcome).toEqual({ total: 1, uploaded: 0, failed: 1 });
    expect(mockUploadAsync).not.toHaveBeenCalled();
  });

  it('keeps the successful photos when one of several fails (partial upload)', async () => {
    primeHappyPath();
    // Second photo's PUT fails; the first and third succeed.
    mockUploadAsync
      .mockResolvedValueOnce({ status: 200 })
      .mockResolvedValueOnce({ status: 500 })
      .mockResolvedValueOnce({ status: 204 });

    const outcome = await uploadFaultPhotos(FAULT_ID, [
      'file:///a.jpg',
      'file:///b.jpg',
      'file:///c.jpg',
    ]);

    expect(outcome).toEqual({ total: 3, uploaded: 2, failed: 1 });
  });

  it('transcodes an iOS HEIC capture to JPEG before uploading', async () => {
    primeHappyPath();
    mockTranscodeToJpeg.mockResolvedValue('file:///converted.jpg');

    const outcome = await uploadFaultPhotos(FAULT_ID, ['file:///IMG_0001.HEIC']);

    expect(outcome.uploaded).toBe(1);
    expect(mockTranscodeToJpeg).toHaveBeenCalledWith('file:///IMG_0001.HEIC');
    // The transcoded JPEG uri (not the original HEIC) is what gets PUT.
    expect(mockUploadAsync).toHaveBeenCalledWith(
      expect.any(String),
      'file:///converted.jpg',
      expect.objectContaining({ headers: { 'Content-Type': 'image/jpeg' } })
    );
    // And the presign was requested as image/jpeg, not image/heic.
    expect(mockApiRequest).toHaveBeenCalledWith(
      '/api/v1/documents/upload-url',
      expect.objectContaining({ body: expect.objectContaining({ mime_type: 'image/jpeg' }) })
    );
  });
});

describe('inferPhotoMeta', () => {
  it('maps a png extension to its MIME type and keeps the filename', async () => {
    const meta = await inferPhotoMeta('file:///cache/photo_9.png');
    expect(meta).toEqual({
      uri: 'file:///cache/photo_9.png',
      fileName: 'photo_9.png',
      mimeType: 'image/png',
    });
  });

  it('defaults an unknown/extensionless uri to jpeg with a generated name', async () => {
    const meta = await inferPhotoMeta('file:///cache/ImageManipulator/tmpABC');
    expect(meta.mimeType).toBe('image/jpeg');
    expect(meta.fileName).toMatch(/^photo_\d+\.jpg$/);
  });

  it('falls back to the original bytes when HEIC transcoding fails', async () => {
    mockTranscodeToJpeg.mockRejectedValue(new Error('manipulator failed'));
    const meta = await inferPhotoMeta('file:///IMG_1.heic');
    // Original uri kept so the upload is still attempted (and rejected by the
    // backend allow-list) rather than silently dropped here.
    expect(meta.uri).toBe('file:///IMG_1.heic');
  });
});
