import type { ScreenMap } from './types.js';

export interface ValidationIssue {
  /**
   * `'warning'` is reserved for non-blocking advisories (e.g. dead links,
   * missing-but-not-required fields). No current rule produces a warning;
   * the lane exists so future rules can be added without an interface
   * change. CLI prints `warn ` for warnings; pre-commit hook only fails
   * on errors.
   */
  severity: 'error' | 'warning';
  path: string;
  message: string;
}

export interface ValidationContext {
  knownEndpointIds: Set<string>;
  knownSitemapIds: Set<string>;
  knownScreenIds: Set<string>;
  /**
   * Resolve a `diagrams[].ref` value. Implementations check filesystem
   * existence and (for `path#anchor`) the presence of the anchor.
   */
  resolveDiagramRef: (ref: string) => boolean;
}

export function validateScreenMap(screen: ScreenMap, ctx: ValidationContext): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  const { frontmatter } = screen;

  // Guard the product/id alignment again at validate time.
  const [productPrefix] = frontmatter.id.split('/');
  if (productPrefix !== frontmatter.product) {
    issues.push({
      severity: 'error',
      path: 'id',
      message: `id prefix "${productPrefix}" does not match product "${frontmatter.product}"`,
    });
  }

  // Endpoints must exist in @ppt/sitemap.
  if (frontmatter.endpoints) {
    frontmatter.endpoints.forEach((endpointId, idx) => {
      if (!ctx.knownEndpointIds.has(endpointId)) {
        issues.push({
          severity: 'error',
          path: `endpoints[${idx}]`,
          message: `unknown endpoint id "${endpointId}" — not present in @ppt/sitemap`,
        });
      }
    });
  }

  // Sitemap refs must exist.
  if (frontmatter.sitemapRefs) {
    for (const [platform, sitemapId] of Object.entries(frontmatter.sitemapRefs)) {
      if (!sitemapId) continue;
      if (!ctx.knownSitemapIds.has(sitemapId)) {
        issues.push({
          severity: 'error',
          path: `sitemapRefs.${platform}`,
          message: `unknown sitemap id "${sitemapId}" — not present in @ppt/sitemap`,
        });
      }
    }
  }

  // Related screens must exist.
  if (frontmatter.relatedScreens) {
    frontmatter.relatedScreens.forEach((rel, idx) => {
      if (!ctx.knownScreenIds.has(rel.id)) {
        issues.push({
          severity: 'error',
          path: `relatedScreens[${idx}].id`,
          message: `related screen "${rel.id}" does not exist`,
        });
      }
    });
  }

  // Diagrams must resolve.
  if (frontmatter.diagrams) {
    frontmatter.diagrams.forEach((diagram, idx) => {
      if (!ctx.resolveDiagramRef(diagram.ref)) {
        issues.push({
          severity: 'error',
          path: `diagrams[${idx}].ref`,
          message: `diagram ref "${diagram.ref}" does not resolve`,
        });
      }
    });
  }

  return issues;
}
