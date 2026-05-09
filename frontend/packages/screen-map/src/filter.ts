import type { ScreenMapFrontmatter } from './types.js';

/**
 * Parse a comma-AND key:value filter expression into a predicate over
 * `ScreenMapFrontmatter`. Supports dotted-path keys (`implementations.ppt-web.buildStatus`)
 * and values containing colons (URL routes like `/buildings/:id`).
 *
 * Empty expression returns a predicate that matches everything.
 */
export function parseFilter(expr: string): (fm: ScreenMapFrontmatter) => boolean {
  if (!expr.trim()) return () => true;
  const terms = expr.split(',').map((t) => {
    const colonIdx = t.indexOf(':');
    return {
      key: (colonIdx >= 0 ? t.slice(0, colonIdx) : t).trim(),
      value: (colonIdx >= 0 ? t.slice(colonIdx + 1) : '').trim(),
    };
  });
  return (fm) => {
    return terms.every(({ key, value }) => {
      const path = key.split('.');
      let cursor: unknown = fm;
      for (const seg of path) {
        if (cursor && typeof cursor === 'object' && seg in cursor) {
          cursor = (cursor as Record<string, unknown>)[seg];
        } else {
          return false;
        }
      }
      return String(cursor) === value;
    });
  };
}
