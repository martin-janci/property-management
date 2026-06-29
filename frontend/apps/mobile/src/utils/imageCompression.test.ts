/**
 * Tests for the offline-fault image-compression helper (Story 4.1, gap #970.6).
 *
 * expo-image-manipulator and expo-file-system/legacy are mocked so the
 * 10 MB-threshold branching is exercised without a native runtime.
 */

const mockGetInfoAsync = jest.fn();
const mockManipulateAsync = jest.fn();

jest.mock('expo-file-system/legacy', () => ({
  getInfoAsync: (...args: unknown[]) => mockGetInfoAsync(...args),
}));

jest.mock('expo-image-manipulator', () => ({
  manipulateAsync: (...args: unknown[]) => mockManipulateAsync(...args),
  SaveFormat: { JPEG: 'jpeg', PNG: 'png', WEBP: 'webp' },
}));

import { bytesToMb, compressImagesIfNeeded, MAX_TOTAL_BYTES } from './imageCompression';

const MB = 1024 * 1024;

beforeEach(() => {
  mockGetInfoAsync.mockReset();
  mockManipulateAsync.mockReset();
});

/** Make getInfoAsync return a fixed size for every uri. */
function sizeEachFile(bytes: number) {
  mockGetInfoAsync.mockResolvedValue({ exists: true, size: bytes });
}

describe('compressImagesIfNeeded', () => {
  it('returns originals untouched when total is at or under 10 MB', async () => {
    sizeEachFile(3 * MB); // 2 photos = 6 MB, under the limit
    const result = await compressImagesIfNeeded(['a.jpg', 'b.jpg']);

    expect(result.compressed).toBe(false);
    expect(result.uris).toEqual(['a.jpg', 'b.jpg']);
    expect(result.originalBytes).toBe(6 * MB);
    expect(mockManipulateAsync).not.toHaveBeenCalled();
  });

  it('compresses every photo when the total exceeds 10 MB', async () => {
    // 3 photos × 5 MB = 15 MB > limit, then each compresses to 1 MB.
    mockGetInfoAsync
      .mockResolvedValueOnce({ exists: true, size: 5 * MB })
      .mockResolvedValueOnce({ exists: true, size: 5 * MB })
      .mockResolvedValueOnce({ exists: true, size: 5 * MB })
      .mockResolvedValue({ exists: true, size: 1 * MB });
    mockManipulateAsync.mockImplementation(async (uri: string) => ({
      uri: `${uri}.compressed`,
      width: 1920,
      height: 1080,
    }));

    const result = await compressImagesIfNeeded(['a.jpg', 'b.jpg', 'c.jpg']);

    expect(result.compressed).toBe(true);
    expect(result.uris).toEqual(['a.jpg.compressed', 'b.jpg.compressed', 'c.jpg.compressed']);
    expect(mockManipulateAsync).toHaveBeenCalledTimes(3);
    expect(result.originalBytes).toBe(15 * MB);
    expect(result.finalBytes).toBe(3 * MB);
  });

  it('keeps the original uri when a single image fails to compress', async () => {
    mockGetInfoAsync.mockResolvedValue({ exists: true, size: 6 * MB }); // 2 × 6 = 12 MB
    mockManipulateAsync
      .mockResolvedValueOnce({ uri: 'a.jpg.compressed', width: 1, height: 1 })
      .mockRejectedValueOnce(new Error('decode failed'));

    const result = await compressImagesIfNeeded(['a.jpg', 'b.jpg']);

    expect(result.compressed).toBe(true);
    expect(result.uris).toEqual(['a.jpg.compressed', 'b.jpg']);
  });

  it('treats an empty selection as nothing to compress', async () => {
    const result = await compressImagesIfNeeded([]);
    expect(result).toEqual({
      uris: [],
      originalBytes: 0,
      finalBytes: 0,
      compressed: false,
      stillOverLimit: false,
    });
    expect(mockGetInfoAsync).not.toHaveBeenCalled();
  });

  it('honours an overridden threshold', async () => {
    sizeEachFile(1 * MB); // 1 MB total
    mockManipulateAsync.mockResolvedValue({ uri: 'a.jpg.c', width: 1, height: 1 });

    const result = await compressImagesIfNeeded(['a.jpg'], { maxTotalBytes: 0.5 * MB });
    expect(result.compressed).toBe(true);
  });

  // issue #1767 §4: pin the manipulateAsync call shape. A `resize` op normalizes
  // orientation and strips EXIF, and JPEG is the upload format — a regression in
  // op order (e.g. dropping the resize) would silently ship rotated images.
  it('compresses with a resize op and re-encodes as JPEG', async () => {
    sizeEachFile(6 * MB); // 2 × 6 = 12 MB > limit
    mockManipulateAsync.mockResolvedValue({ uri: 'out.jpg', width: 1920, height: 1080 });

    await compressImagesIfNeeded(['a.jpg', 'b.jpg']);

    expect(mockManipulateAsync).toHaveBeenCalledWith(
      'a.jpg',
      [{ resize: { width: 1920 } }],
      expect.objectContaining({ format: 'jpeg', compress: expect.any(Number) })
    );
  });

  // issue #1767 §3: a single fixed-quality pass is not guaranteed to land under
  // the limit. When the compressed total is still over, the result must say so
  // (stillOverLimit) so the UI warns the upload may be slow/fail.
  it('signals stillOverLimit when the compressed total still exceeds the limit', async () => {
    // 3 × 5 MB = 15 MB originals (> 10 MB → compress); each "compresses" to
    // 4 MB, so 12 MB total still exceeds the 10 MB limit.
    mockGetInfoAsync
      .mockResolvedValueOnce({ exists: true, size: 5 * MB })
      .mockResolvedValueOnce({ exists: true, size: 5 * MB })
      .mockResolvedValueOnce({ exists: true, size: 5 * MB })
      .mockResolvedValue({ exists: true, size: 4 * MB });
    mockManipulateAsync.mockImplementation(async (uri: string) => ({
      uri: `${uri}.c`,
      width: 1920,
      height: 1080,
    }));

    const result = await compressImagesIfNeeded(['a.jpg', 'b.jpg', 'c.jpg']);

    expect(result.compressed).toBe(true);
    expect(result.finalBytes).toBe(12 * MB);
    expect(result.stillOverLimit).toBe(true);
  });

  it('reports stillOverLimit=false when compression brings the total under the limit', async () => {
    mockGetInfoAsync
      .mockResolvedValueOnce({ exists: true, size: 6 * MB })
      .mockResolvedValueOnce({ exists: true, size: 6 * MB }) // 12 MB originals
      .mockResolvedValue({ exists: true, size: 1 * MB }); // 2 MB after compression
    mockManipulateAsync.mockImplementation(async (uri: string) => ({
      uri: `${uri}.c`,
      width: 1,
      height: 1,
    }));

    const result = await compressImagesIfNeeded(['a.jpg', 'b.jpg']);

    expect(result.compressed).toBe(true);
    expect(result.stillOverLimit).toBe(false);
  });
});

describe('bytesToMb', () => {
  it('formats bytes as one-decimal megabytes', () => {
    expect(bytesToMb(MAX_TOTAL_BYTES)).toBe('10.0');
    expect(bytesToMb(15 * MB)).toBe('15.0');
    expect(bytesToMb(1.55 * MB)).toBe('1.6');
  });
});
