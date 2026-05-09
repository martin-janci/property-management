export interface DesignFrame {
  id: string;
  /** Human-readable name shown in the review UI. */
  name: string;
  /** URL the SPA fetches to render the image. Server resolves to a stream. */
  imageUrl: string;
  /** Pixel dimensions for layout calculations. */
  width: number;
  height: number;
  /** Adapter-specific extras (e.g. ZIP file path, Figma node id). */
  metadata?: Record<string, unknown>;
}

export interface DesignSource {
  /** Stable identifier — used as the `:adapter` segment of /api/designs/. */
  readonly name: string;
  list(): Promise<DesignFrame[]>;
  get(id: string): Promise<DesignFrame | null>;
  /** Optional: stream raw bytes for a frame. Server uses this to proxy images. */
  readBytes?(id: string): Promise<Uint8Array | null>;
}

export interface DesignSourceConfig {
  adapter: 'zip' | 'claude-design';
  /** ZipAdapter: path to .zip file (relative to repoRoot or absolute). */
  file?: string;
  [key: string]: unknown;
}

/**
 * Build a DesignSource from a config record, typically read from frontmatter
 * `designSources[]` or from a screen-map config.
 *
 * Throws on unknown adapter names so misconfiguration is loud.
 */
export async function createDesignSource(
  config: DesignSourceConfig,
  context: { repoRoot: string }
): Promise<DesignSource> {
  switch (config.adapter) {
    case 'zip': {
      if (!config.file) {
        throw new Error('zip adapter requires a "file" config key');
      }
      const { ZipAdapter } = await import('./zip-adapter.js');
      return ZipAdapter.fromFile(config.file, context.repoRoot);
    }
    case 'claude-design': {
      const { ClaudeDesignAdapter } = await import('./claude-design.js');
      return new ClaudeDesignAdapter();
    }
    default: {
      const adapter: never = config.adapter;
      throw new Error(`unknown DesignSource adapter: ${String(adapter)}`);
    }
  }
}
