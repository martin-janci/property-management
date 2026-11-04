/**
 * Cross-stack drift guard for the vote eligibility-reason i18n mapping
 * (GitHub #3223, follow-up to PR #3222).
 *
 * `StatusBanner` localizes the backend's eligibility `reason` by mapping its
 * English free-text to a stable i18n key suffix via `ELIGIBILITY_REASON_KEYS`
 * (VoteDetailScreen.tsx). The source of truth for those sentences is the Rust
 * backend — `backend/crates/db/src/repositories/vote.rs`, the `let reason = …`
 * block of `check_eligibility`. Because the client passes the raw string as
 * `t(…, { defaultValue })`, a backend copy-edit (e.g. "User is not eligible to
 * vote in this building" → "You are not eligible …") would NOT crash, fail
 * typecheck, or fail any other test: the mapping would silently fall through to
 * the slug branch and a non-English voter would silently lose the translation.
 *
 * This test parses the Rust single-source-of-truth literals and fails if the TS
 * map diverges in EITHER direction:
 *   - a backend reason the map doesn't key off → unmapped (lost translation),
 *   - a map key no longer present in the backend → dead mapping (stale copy).
 * So a backend change is forced to come with a matching mobile change (or an
 * explicit acknowledgement here). Reads the Rust source directly (no build
 * step, no generated fixture) so the guard tracks the actual source of truth,
 * mirroring DocumentUploadScreen.parity.test.ts (GH #2368).
 */

import { ELIGIBILITY_REASON_KEYS } from './VoteDetailScreen';

// This suite runs under Node (jest), but the React Native `tsconfig` doesn't
// pull in `@types/node`, so declare the tiny slice of the Node API we use here
// rather than adding a project-wide dependency just for one test. `require`
// resolves at runtime because babel-jest compiles this module to CommonJS.
declare const __dirname: string;
declare function require(id: 'node:fs'): { readFileSync(path: string, encoding: 'utf8'): string };
declare function require(id: 'node:path'): { join(...parts: string[]): string };

const { readFileSync } = require('node:fs');
const { join } = require('node:path');

// voting/ → screens → src → mobile → apps → frontend → repo root
const REPO_ROOT = join(__dirname, '../../../../../..');
const VOTE_RS = join(REPO_ROOT, 'backend/crates/db/src/repositories/vote.rs');

/**
 * Extract the eligibility reason sentences from the `let reason = if … };`
 * block of `check_eligibility` — i.e. every `Some("…".to_string())` literal.
 * Scoping to that assignment keeps unrelated `"…".to_string()` elsewhere in the
 * ~1500-line file (cancellation reasons, comment-hide reasons, …) out of scope.
 */
function parseRustEligibilityReasons(source: string): string[] {
  const block = source.match(/let reason\s*=\s*if[\s\S]*?\n\s*};/);
  if (!block) {
    throw new Error(
      `Could not locate the \`let reason = if … };\` block in ${VOTE_RS}. ` +
        'If check_eligibility moved or was refactored, update this drift guard (GH #3223).'
    );
  }
  return [...block[0].matchAll(/Some\("([^"]+)"\.to_string\(\)\)/g)].map((m) => m[1]);
}

describe('VoteDetailScreen ↔ backend eligibility-reason parity (GH #3223)', () => {
  it('ELIGIBILITY_REASON_KEYS mirrors the Rust check_eligibility reasons exactly', () => {
    const rustReasons = parseRustEligibilityReasons(readFileSync(VOTE_RS, 'utf8'));

    // Sanity: the parse found the known three (guards against a silently empty
    // match that would make the equality below vacuously pass).
    expect(rustReasons.length).toBeGreaterThanOrEqual(3);

    // Every backend reason must be mapped (else a non-English voter silently
    // loses the translation), and every mapped key must still exist on the
    // backend (else it's dead copy). Compared as sets, since neither side is an
    // ordered sequence.
    expect([...rustReasons].sort()).toEqual([...Object.keys(ELIGIBILITY_REASON_KEYS)].sort());
  });
});
