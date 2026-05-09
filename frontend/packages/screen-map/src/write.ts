import { writeFile } from 'node:fs/promises';
import matter from 'gray-matter';
import type { ScreenMap } from './types.js';

/**
 * Serialises a ScreenMap back to markdown. The frontmatter is regenerated from
 * the typed object (ordering is whatever gray-matter chooses); the body is
 * preserved exactly as supplied.
 */
export function writeScreenMapString(screen: ScreenMap): string {
  const yaml = matter.stringify('', screen.frontmatter, {
    language: 'yaml',
  });
  // gray-matter's stringify produces "---\n<yaml>\n---\n" with an extra
  // trailing newline; we want the body to start exactly one blank line below
  // the closing fence.
  const trimmed = yaml.replace(/\n+$/, '');
  return `${trimmed}\n\n${screen.body}`;
}

export async function writeScreenMap(screen: ScreenMap): Promise<void> {
  const serialised = writeScreenMapString(screen);
  await writeFile(screen.filePath, serialised, 'utf8');
}
