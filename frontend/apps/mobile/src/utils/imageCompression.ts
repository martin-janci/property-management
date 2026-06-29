/**
 * Image compression for offline fault reporting (Story 4.1, gap #970.6).
 *
 * AC 4.1 requires that when selected photos exceed 10 MB in total they are
 * compressed before they are attached to a fault report, and the user is shown
 * a quality-loss warning. Phone cameras routinely produce 4–8 MB images, so a
 * handful of photos blow past the limit and would make uploads (especially on
 * a flaky/offline connection that later replays from the queue) slow and
 * failure-prone.
 *
 * The strategy is a single down-scale + re-encode pass per image: cap the
 * longest edge at {@link MAX_DIMENSION}px and re-encode as JPEG at
 * {@link DEFAULT_QUALITY}. This is the standard expo-image-manipulator recipe
 * and shrinks multi-megapixel camera shots by an order of magnitude while
 * leaving the photos legible for a maintenance technician.
 */
import * as FileSystem from 'expo-file-system/legacy';
import { manipulateAsync, SaveFormat } from 'expo-image-manipulator';

/** The 10 MB total-size threshold from AC 4.1. */
export const MAX_TOTAL_BYTES = 10 * 1024 * 1024;

/** Longest-edge cap applied when compressing. */
const MAX_DIMENSION = 1920;

/** JPEG quality (0–1) applied when compressing. */
const DEFAULT_QUALITY = 0.6;

export interface CompressionResult {
  /** Photo URIs — compressed copies when {@link compressed} is true, otherwise the originals. */
  uris: string[];
  /** Combined size of the original photos, in bytes. */
  originalBytes: number;
  /** Combined size after the (possible) compression pass, in bytes. */
  finalBytes: number;
  /** True when the photos exceeded the limit and a compression pass ran. */
  compressed: boolean;
  /**
   * True when the photos still exceed the limit *after* the compression pass
   * (issue #1767). The UI can use this to warn that the upload may be slow /
   * fail rather than implying compression brought it under the threshold.
   */
  stillOverLimit: boolean;
}

export interface CompressionOptions {
  /** Override the total-size threshold (defaults to {@link MAX_TOTAL_BYTES}). */
  maxTotalBytes?: number;
  /** Override the longest-edge cap (defaults to {@link MAX_DIMENSION}). */
  maxDimension?: number;
  /** Override the JPEG quality (defaults to {@link DEFAULT_QUALITY}). */
  quality?: number;
}

/**
 * Max number of images decoded/re-encoded at once (issue #1767).
 *
 * The previous implementation ran every image through `manipulateAsync` via
 * `Promise.all`, decoding up to 5 multi-megapixel photos simultaneously — a
 * memory spike that can OOM low-end devices. Capping concurrency at 2 bounds
 * peak memory while still overlapping a little I/O.
 */
const COMPRESSION_CONCURRENCY = 2;

/**
 * Map over `items` with at most `limit` invocations of `fn` in flight at once,
 * preserving input order in the result. A tiny bounded-concurrency runner so we
 * don't pull in a dependency for one use.
 */
async function mapWithConcurrency<T, R>(
  items: T[],
  limit: number,
  fn: (item: T, index: number) => Promise<R>
): Promise<R[]> {
  const results = new Array<R>(items.length);
  let next = 0;
  const workerCount = Math.max(1, Math.min(limit, items.length));

  async function worker(): Promise<void> {
    while (true) {
      const index = next++;
      if (index >= items.length) return;
      results[index] = await fn(items[index], index);
    }
  }

  await Promise.all(Array.from({ length: workerCount }, () => worker()));
  return results;
}

/** Best-effort size of a single local file in bytes; 0 if it can't be read. */
async function fileSize(uri: string): Promise<number> {
  try {
    // Legacy `getInfoAsync` returns `size` on the `exists: true` branch of the
    // FileInfo union; no option is needed to opt into it.
    const info = await FileSystem.getInfoAsync(uri);
    return info.exists ? info.size : 0;
  } catch {
    // A missing/unreadable file contributes 0 to the total rather than
    // aborting the whole size check.
    return 0;
  }
}

/** Combined byte size of a set of local image URIs. */
export async function totalImageBytes(uris: string[]): Promise<number> {
  if (uris.length === 0) return 0;
  const sizes = await Promise.all(uris.map(fileSize));
  return sizes.reduce((sum, size) => sum + size, 0);
}

/** Bytes → megabytes, rounded to one decimal, for user-facing copy. */
export function bytesToMb(bytes: number): string {
  return (bytes / (1024 * 1024)).toFixed(1);
}

/**
 * Compress `uris` only when their combined size exceeds the limit.
 *
 * When under the limit the originals are returned untouched (`compressed:
 * false`) so we never degrade quality needlessly. When over, every image is
 * down-scaled + re-encoded; a single image that fails to process falls back to
 * its original URI rather than dropping the photo.
 */
export async function compressImagesIfNeeded(
  uris: string[],
  options: CompressionOptions = {}
): Promise<CompressionResult> {
  const maxTotalBytes = options.maxTotalBytes ?? MAX_TOTAL_BYTES;
  const originalBytes = await totalImageBytes(uris);

  if (uris.length === 0 || originalBytes <= maxTotalBytes) {
    return {
      uris,
      originalBytes,
      finalBytes: originalBytes,
      compressed: false,
      stillOverLimit: false,
    };
  }

  const maxDimension = options.maxDimension ?? MAX_DIMENSION;
  const quality = options.quality ?? DEFAULT_QUALITY;

  // Compress with bounded concurrency (cap of 2) rather than all-at-once, so
  // peak memory stays in check on low-end devices (issue #1767). Order is
  // preserved so each output lines up with its input uri.
  const compressedUris = await mapWithConcurrency(uris, COMPRESSION_CONCURRENCY, async (uri) => {
    try {
      const result = await manipulateAsync(uri, [{ resize: { width: maxDimension } }], {
        compress: quality,
        format: SaveFormat.JPEG,
      });
      return result.uri;
    } catch {
      // Keep the original on a per-image failure so the report still has the photo.
      return uri;
    }
  });

  const finalBytes = await totalImageBytes(compressedUris);
  // A single fixed-quality pass is not guaranteed to land under the limit (many
  // large photos, or photos that don't shrink much). Surface that so the UI can
  // warn the upload may still be slow / fail instead of implying it's now fine.
  const stillOverLimit = finalBytes > maxTotalBytes;
  return { uris: compressedUris, originalBytes, finalBytes, compressed: true, stillOverLimit };
}
