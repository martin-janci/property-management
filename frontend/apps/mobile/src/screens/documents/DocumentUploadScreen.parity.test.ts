/**
 * Cross-stack drift guard for the client-side document-upload limits
 * (GitHub #2368).
 *
 * `DocumentUploadScreen` enforces `MAX_FILE_SIZE` / `ALLOWED_MIME_TYPES`
 * *before* streaming the multipart body, so the client blocks first. That makes
 * the client the effective gate: if the backend ever **widens** its allow-list
 * (e.g. adds `image/heic` for iOS photos) but this hand-maintained TS mirror is
 * not updated, the app would silently reject a file the server now accepts — a
 * false-rejection UX regression with no other test or CI signal.
 *
 * This test parses the Rust single-source-of-truth constants and fails if the
 * TS copy diverges, so a backend change forces a matching mobile change (or an
 * explicit acknowledgement here). A *narrowing* is harmless — the server 422 is
 * the backstop — but the guard is symmetric: any divergence trips it, which is
 * the cheap, honest thing to do.
 *
 * Reads the Rust sources directly (no build step, no generated fixture) so the
 * guard tracks the actual source of truth:
 *   - MIME list  → backend/crates/common/src/media.rs (ALLOWED_UPLOAD_MIME_TYPES)
 *   - size cap   → backend/crates/db/src/models/document.rs (MAX_FILE_SIZE)
 */

import { ALLOWED_MIME_TYPES, MAX_FILE_SIZE } from './DocumentUploadScreen';

// This suite runs under Node (jest), but the React Native `tsconfig` doesn't
// pull in `@types/node`, so declare the tiny slice of the Node API we use here
// rather than adding a project-wide dependency just for one test. `require`
// resolves at runtime because babel-jest compiles this module to CommonJS.
declare const __dirname: string;
declare function require(id: 'node:fs'): { readFileSync(path: string, encoding: 'utf8'): string };
declare function require(id: 'node:path'): { join(...parts: string[]): string };

const { readFileSync } = require('node:fs');
const { join } = require('node:path');

// documents/ → screens → src → mobile → apps → frontend → repo root
const REPO_ROOT = join(__dirname, '../../../../../..');
const MEDIA_RS = join(REPO_ROOT, 'backend/crates/common/src/media.rs');
const DOCUMENT_RS = join(REPO_ROOT, 'backend/crates/db/src/models/document.rs');

/** Extract the string literals inside `ALLOWED_UPLOAD_MIME_TYPES: &[&str] = &[ ... ];`. */
function parseRustMimeList(source: string): string[] {
  const block = source.match(/ALLOWED_UPLOAD_MIME_TYPES\s*:\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/);
  if (!block) {
    throw new Error(
      `Could not locate ALLOWED_UPLOAD_MIME_TYPES in ${MEDIA_RS}. ` +
        'If the constant moved or was renamed, update this drift guard (GH #2368).'
    );
  }
  return [...block[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
}

/** Evaluate the `MAX_FILE_SIZE: i64 = <expr>;` product (e.g. `50 * 1024 * 1024`). */
function parseRustMaxFileSize(source: string): number {
  const match = source.match(/MAX_FILE_SIZE\s*:\s*i64\s*=\s*([^;]+);/);
  if (!match) {
    throw new Error(
      `Could not locate MAX_FILE_SIZE in ${DOCUMENT_RS}. ` +
        'If the constant moved or was renamed, update this drift guard (GH #2368).'
    );
  }
  return match[1]
    .split('*')
    .map((factor) => Number.parseInt(factor.trim(), 10))
    .reduce((product, n) => product * n, 1);
}

describe('DocumentUploadScreen ↔ backend upload-limit parity (GH #2368)', () => {
  it('ALLOWED_MIME_TYPES matches the Rust ALLOWED_UPLOAD_MIME_TYPES source of truth', () => {
    const rustList = parseRustMimeList(readFileSync(MEDIA_RS, 'utf8'));

    // Sanity: the parse found a non-trivial list (guards against a silently
    // empty match that would make the equality below vacuously pass).
    expect(rustList.length).toBeGreaterThanOrEqual(5);

    // Order-insensitive: both sides are allow-lists, not ordered sequences.
    expect([...ALLOWED_MIME_TYPES].sort()).toEqual([...rustList].sort());
  });

  it('MAX_FILE_SIZE matches the Rust MAX_FILE_SIZE source of truth', () => {
    const rustMax = parseRustMaxFileSize(readFileSync(DOCUMENT_RS, 'utf8'));

    expect(rustMax).toBeGreaterThan(0);
    expect(MAX_FILE_SIZE).toBe(rustMax);
  });
});
