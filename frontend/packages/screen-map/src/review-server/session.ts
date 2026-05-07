import { randomBytes } from 'node:crypto';
import type { DesignSource } from '../design-source/index.js';
import type { ScreenMap } from '../types.js';

export interface ReviewSession {
  /** 32-hex-char token; required as `?session=<token>` for all API calls. */
  readonly token: string;
  /** Filtered, ordered list of screens for this review walk. */
  screens: ScreenMap[];
  /** Index into `screens`; mutated as user navigates. */
  currentIdx: number;
  /** Resolved DesignSource per `name` (zip / claude-design / ...). */
  designSources: Map<string, DesignSource>;
  /** `--preview` flag passthrough: where the right pane points by default. */
  defaultPreview: 'local' | 'staging' | 'design';
}

export function createSession(args: {
  screens: ScreenMap[];
  designSources?: DesignSource[];
  defaultPreview?: ReviewSession['defaultPreview'];
}): ReviewSession {
  const sources = new Map<string, DesignSource>();
  for (const ds of args.designSources ?? []) sources.set(ds.name, ds);
  return {
    token: randomBytes(16).toString('hex'),
    screens: args.screens,
    currentIdx: 0,
    designSources: sources,
    defaultPreview: args.defaultPreview ?? 'local',
  };
}
