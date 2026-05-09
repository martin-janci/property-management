/**
 * Two products covered by the screen-map system.
 * - `ppt`: Property Management (ppt-web + mobile)
 * - `reality`: Reality Portal (reality-web + mobile-native)
 */
export type Product = 'ppt' | 'reality';

/**
 * All platforms across both products. A given screen will only use the
 * platforms relevant to its product (ppt → ppt-web/mobile;
 * reality → reality-web/mobile-native).
 */
export type Platform = 'ppt-web' | 'reality-web' | 'mobile' | 'mobile-native';

export type BuildStatus = 'planned' | 'in-progress' | 'shipped' | 'n/a';
export type RedesignStatus = 'not-started' | 'in-progress' | 'applied' | 'n/a';
export type ApiStatus = 'stub' | 'partial' | 'complete' | 'n/a';

export type RelatedRel = 'parent' | 'child' | 'action' | 'sibling';
export type DiagramKind = 'sequence' | 'flow' | 'state' | 'class';

export interface Implementation {
  /** URL pattern for ppt-web/reality-web; absent on mobile platforms. */
  route?: string;
  /** Native screen name for mobile/mobile-native; absent on web. */
  screen?: string;
  /** React component or KMP screen class. */
  component?: string;
  buildStatus: BuildStatus;
  redesignStatus: RedesignStatus;
  apiStatus: ApiStatus;
}

export interface RelatedScreen {
  id: string;
  rel: RelatedRel;
}

export interface DiagramRef {
  /** Path or anchor; e.g. `docs/sequence-diagrams.md#building-detail-load`. */
  ref: string;
  kind: DiagramKind;
}

export interface DesignSourceRef {
  adapter: string;
  /** Adapter-specific. ZipAdapter uses `file` + `frame`. */
  file?: string;
  frame: string;
  [key: string]: unknown;
}

export interface ScreenMapFrontmatter {
  id: string;
  name: string;
  product: Product;
  sitemapRefs?: Partial<Record<Platform, string>>;
  implementations: Partial<Record<Platform, Implementation>>;
  endpoints?: string[];
  relatedScreens?: RelatedScreen[];
  sharedComponents?: string[];
  diagrams?: DiagramRef[];
  useCases?: string[];
  epics?: string[];
  designSources?: DesignSourceRef[];
  owner?: string;
  /** ISO date YYYY-MM-DD. */
  lastReview?: string;
}

export interface ScreenMap {
  /** Absolute or repo-relative path of the source markdown file. */
  filePath: string;
  frontmatter: ScreenMapFrontmatter;
  /** Markdown body (everything after the closing frontmatter delimiter). */
  body: string;
}
