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
}

export interface CompressionOptions {
  /** Override the total-size threshold (defaults to {@link MAX_TOTAL_BYTES}). */
  maxTotalBytes?: number;
  /** Override the longest-edge cap (defaults to {@link MAX_DIMENSION}). */
  maxDimension?: number;
  /** Override the JPEG quality (defaults to {@link DEFAULT_QUALITY}). */
  quality?: number;
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
    return { uris, originalBytes, finalBytes: originalBytes, compressed: false };
  }

  const maxDimension = options.maxDimension ?? MAX_DIMENSION;
  const quality = options.quality ?? DEFAULT_QUALITY;

  const compressedUris = await Promise.all(
    uris.map(async (uri) => {
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
    })
  );

  const finalBytes = await totalImageBytes(compressedUris);
  return { uris: compressedUris, originalBytes, finalBytes, compressed: true };
}
