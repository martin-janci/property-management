/**
 * meterPhotoUpload — presigned meter-photo upload pipeline
 * (code-review: mobile-rn-meter-photo-not-uploaded).
 *
 * Regression guard: the captured meter photo used to be queued only as a local
 * `file://` URI, so the api-server never received the bytes and the photo was
 * silently dropped. This suite pins the two network steps that now actually
 * upload it and return the storage key for `SubmitReading.photo_url`:
 *
 *   1. request a presigned URL   → POST /api/v1/documents/upload-url
 *   2. PUT the bytes to storage  → FileSystem.uploadAsync (BINARY_CONTENT, PUT)
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
import { uploadMeterPhoto } from './meterPhotoUpload';

const mockApiRequest = apiRequest as jest.Mock;

/** Wire apiRequest to answer the upload-url POST with a presigned response. */
function primeHappyPath(fileKey = 'org-1/2026/09/abc_meter.jpg') {
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
    throw new Error(`unexpected path ${path}`);
  });
}

beforeEach(() => {
  jest.clearAllMocks();
  mockGetInfoAsync.mockResolvedValue({ exists: true, size: 54321 });
  mockUploadAsync.mockResolvedValue({ status: 200, body: '' });
});

describe('uploadMeterPhoto', () => {
  it('runs presign → PUT and returns the storage key for photo_url', async () => {
    primeHappyPath();

    const key = await uploadMeterPhoto('file:///meter.jpg');

    // The returned key is exactly what the reading submission sends as photo_url.
    expect(key).toBe('org-1/2026/09/abc_meter.jpg');

    // Step 1 — presigned URL request carries filename, mime and the real size.
    expect(mockApiRequest).toHaveBeenCalledWith('/api/v1/documents/upload-url', {
      method: 'POST',
      body: { file_name: 'meter.jpg', mime_type: 'image/jpeg', size_bytes: 54321 },
    });

    // Step 2 — bytes PUT straight to the presigned URL with the signed
    // Content-Type and no auth headers.
    expect(mockUploadAsync).toHaveBeenCalledWith('https://s3.example/put?sig=xyz', 'file:///meter.jpg', {
      httpMethod: 'PUT',
      uploadType: 0,
      headers: { 'Content-Type': 'image/jpeg' },
    });
  });

  it('throws when the S3 PUT fails so the caller never keeps a bogus key', async () => {
    primeHappyPath();
    mockUploadAsync.mockResolvedValue({ status: 403, body: 'AccessDenied' });

    await expect(uploadMeterPhoto('file:///meter.jpg')).rejects.toThrow(/403/);
  });

  it('throws when the presign request rejects and never PUTs bytes', async () => {
    mockApiRequest.mockRejectedValue(new Error('HTTP 503'));

    await expect(uploadMeterPhoto('file:///meter.jpg')).rejects.toThrow();
    expect(mockUploadAsync).not.toHaveBeenCalled();
  });

  it('transcodes an iOS HEIC capture to JPEG before uploading', async () => {
    primeHappyPath();
    mockTranscodeToJpeg.mockResolvedValue('file:///converted.jpg');

    await uploadMeterPhoto('file:///IMG_0001.HEIC');

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
