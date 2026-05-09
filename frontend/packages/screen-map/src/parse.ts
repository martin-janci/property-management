import { readFile } from 'node:fs/promises';
import matter from 'gray-matter';
import { ScreenMapFrontmatterSchema } from './schema.js';
import type { ScreenMap } from './types.js';

export class ScreenMapParseError extends Error {
  constructor(
    public readonly filePath: string,
    public readonly issues: string[]
  ) {
    super(`Invalid screen-map at ${filePath}:\n  - ${issues.join('\n  - ')}`);
    this.name = 'ScreenMapParseError';
  }
}

export function parseScreenMapString(source: string, filePath: string): ScreenMap {
  const parsed = matter(source);
  const result = ScreenMapFrontmatterSchema.safeParse(parsed.data);
  if (!result.success) {
    const issues = result.error.issues.map((i) => {
      const path = i.path.join('.') || '<root>';
      return `${path}: ${i.message}`;
    });
    throw new ScreenMapParseError(filePath, issues);
  }
  return {
    filePath,
    frontmatter: result.data,
    body: parsed.content.replace(/^\r?\n/, ''),
  };
}

export async function parseScreenMap(filePath: string): Promise<ScreenMap> {
  const source = await readFile(filePath, 'utf8');
  return parseScreenMapString(source, filePath);
}
