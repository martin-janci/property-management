import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { ZipAdapter } from '../../src/design-source/zip-adapter.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.join(here, '..', 'fixtures', 'designs-2026-q2.zip');

describe('ZipAdapter', () => {
  it('reads manifest.json and lists frames', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const frames = await adapter.list();
    expect(frames).toHaveLength(2);
    const webFrame = frames.find((f) => f.id === 'building-detail-v3-web');
    expect(webFrame).toBeDefined();
    expect(webFrame?.width).toBe(1440);
    expect(webFrame?.imageUrl).toContain('zip/building-detail-v3-web');
  });

  it('returns null for an unknown frame id', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const frame = await adapter.get('does-not-exist');
    expect(frame).toBeNull();
  });

  it('streams frame bytes via readBytes', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const bytes = await adapter.readBytes!('building-detail-v3-web');
    expect(bytes).toBeInstanceOf(Uint8Array);
    // PNG signature.
    expect(bytes![0]).toBe(0x89);
    expect(bytes![1]).toBe(0x50);
  });

  it('returns null bytes for unknown frame', async () => {
    const adapter = await ZipAdapter.fromFile(fixture, '/');
    const bytes = await adapter.readBytes!('does-not-exist');
    expect(bytes).toBeNull();
  });
});
