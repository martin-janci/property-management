import path from 'node:path';
import type { Readable } from 'node:stream';
import { open as openZip } from 'yauzl-promise';
import type { DesignFrame, DesignSource } from './index.js';

interface ManifestEntry {
  id: string;
  name: string;
  file: string;
  width: number;
  height: number;
}

interface Manifest {
  frames: ManifestEntry[];
}

export class ZipAdapter implements DesignSource {
  readonly name = 'zip';

  private constructor(
    private readonly zipPath: string,
    private readonly manifest: Manifest,
    /** in-memory byte cache; first read populates, later reads hit. */
    private readonly cache: Map<string, Uint8Array>
  ) {}

  static async fromFile(filePart: string, repoRoot: string): Promise<ZipAdapter> {
    const zipPath = path.isAbsolute(filePart) ? filePart : path.join(repoRoot, filePart);
    const zip = await openZip(zipPath);
    let manifest: Manifest | null = null;
    try {
      for await (const entry of zip) {
        if (entry.filename === 'manifest.json') {
          const stream = await entry.openReadStream();
          const buf = await streamToBuffer(stream);
          manifest = JSON.parse(buf.toString('utf8')) as Manifest;
          break;
        }
      }
    } finally {
      await zip.close();
    }
    if (!manifest) {
      throw new Error(`ZipAdapter: ${zipPath} has no manifest.json at the root`);
    }
    if (!Array.isArray(manifest.frames)) {
      throw new Error('ZipAdapter: manifest.frames must be an array');
    }
    return new ZipAdapter(zipPath, manifest, new Map());
  }

  async list(): Promise<DesignFrame[]> {
    return this.manifest.frames.map((f) => this.toFrame(f));
  }

  async get(id: string): Promise<DesignFrame | null> {
    const entry = this.manifest.frames.find((f) => f.id === id);
    return entry ? this.toFrame(entry) : null;
  }

  async readBytes(id: string): Promise<Uint8Array | null> {
    const cached = this.cache.get(id);
    if (cached) return cached;
    const entry = this.manifest.frames.find((f) => f.id === id);
    if (!entry) return null;

    const zip = await openZip(this.zipPath);
    try {
      for await (const e of zip) {
        if (e.filename === entry.file) {
          const stream = await e.openReadStream();
          const buf = await streamToBuffer(stream);
          const bytes = new Uint8Array(buf);
          this.cache.set(id, bytes);
          return bytes;
        }
      }
    } finally {
      await zip.close();
    }
    return null;
  }

  private toFrame(entry: ManifestEntry): DesignFrame {
    return {
      id: entry.id,
      name: entry.name,
      imageUrl: `/api/designs/zip/${encodeURIComponent(entry.id)}`,
      width: entry.width,
      height: entry.height,
      metadata: { sourceFile: this.zipPath, frameFile: entry.file },
    };
  }
}

async function streamToBuffer(stream: Readable): Promise<Buffer> {
  const chunks: Buffer[] = [];
  for await (const chunk of stream) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }
  return Buffer.concat(chunks);
}
