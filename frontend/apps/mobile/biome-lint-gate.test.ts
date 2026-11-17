/**
 * Dated flip-back guard for the mobile warn-only lint rollout (issue #3584).
 *
 * PR #3566 relaxed two correctness rules in `biome.json`
 * (`useHookAtTopLevel`, `useExhaustiveDependencies`) and shipped the
 * `no-hardcoded-strings.grit` i18n plugin at `warn`, "warn-only for one
 * sprint" to let the existing backlog be migrated without failing the gate.
 *
 * The problem (issue #3584): that "flip all three back to `error` next
 * sprint" promise lived ONLY as prose in `CLAUDE.md` — no date, no issue,
 * no CI guard — so the overwhelmingly likely outcome was the warn-only state
 * becoming permanent and the gate quietly eroding.
 *
 * This suite is the tracked follow-up mechanism that replaces the prose
 * reminder. It:
 *   1. pins that the relaxed rules + the grit plugin stay *present* (so a
 *      future refactor can't silently drop the diagnostics entirely), and
 *   2. converts "flip back next sprint" into an enforceable deadline — once
 *      `FLIP_BACK_DEADLINE` passes, the severities MUST be back at `error`
 *      or this test fails CI, pointing the owner straight at issue #3584.
 *
 * Config + docs only: this guard reads the committed `biome.json` /
 * `.grit` files; it does not touch app logic. When the backlog is clear,
 * promote the three severities to `error` (here and in the grit plugin) —
 * that makes the deadline branch below pass. If more time is genuinely
 * needed, extend `FLIP_BACK_DEADLINE` *with a justification recorded on
 * issue #3584* rather than deleting this guard.
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const BIOME_CONFIG = join(__dirname, 'biome.json');
const GRIT_PLUGIN = join(__dirname, 'biome-plugins', 'no-hardcoded-strings.grit');

/**
 * Promotion deadline for the warn-only rollout. PR #3566 merged
 * 2026-11-16 and granted "one sprint"; this is the end of that window.
 * After this date the relaxed rules must be back at `error`. See #3584.
 */
const FLIP_BACK_DEADLINE = '2026-12-01';

/** correctness rules PR #3566 relaxed from `error` to `warn`. */
const WARN_ONLY_RULES = ['useHookAtTopLevel', 'useExhaustiveDependencies'] as const;

function readCorrectnessRules(): Record<string, unknown> {
  const biome = JSON.parse(readFileSync(BIOME_CONFIG, 'utf8'));
  return biome?.linter?.rules?.correctness ?? {};
}

/** Every `severity="..."` emitted by the grit i18n plugin. */
function readGritSeverities(): string[] {
  const grit = readFileSync(GRIT_PLUGIN, 'utf8');
  return [...grit.matchAll(/severity="(\w+)"/g)].map((m) => m[1]);
}

function pastDeadline(): boolean {
  return Date.now() >= Date.parse(`${FLIP_BACK_DEADLINE}T00:00:00Z`);
}

describe('mobile biome warn-only lint rollout (issue #3584)', () => {
  const correctness = readCorrectnessRules();
  const gritSeverities = readGritSeverities();

  it('keeps the relaxed correctness rules present in biome.json', () => {
    for (const rule of WARN_ONLY_RULES) {
      // Must stay declared (warn during the window, error after) — a silent
      // deletion would reopen the lint gap PR #3566 set out to close.
      expect(['warn', 'error']).toContain(correctness[rule]);
    }
  });

  it('keeps the grit i18n plugin emitting diagnostics', () => {
    expect(gritSeverities.length).toBeGreaterThan(0);
    for (const sev of gritSeverities) {
      expect(['warn', 'error']).toContain(sev);
    }
  });

  // The enforcement. Before the deadline this is a no-op pass; once the
  // sprint window closes it fails until the severities are promoted back to
  // `error`, so the flip-back cannot be silently forgotten. See issue #3584.
  it(`promotes the relaxed rules back to "error" once past ${FLIP_BACK_DEADLINE}`, () => {
    if (!pastDeadline()) {
      // Within the warn-only sprint window — nothing to enforce yet.
      return;
    }

    const stillRelaxed: string[] = [];
    for (const rule of WARN_ONLY_RULES) {
      if (correctness[rule] !== 'error') {
        stillRelaxed.push(`biome.json correctness.${rule} = "${correctness[rule]}"`);
      }
    }
    for (const [i, sev] of gritSeverities.entries()) {
      if (sev !== 'error') {
        stillRelaxed.push(`no-hardcoded-strings.grit severity[#${i}] = "${sev}"`);
      }
    }

    if (stillRelaxed.length > 0) {
      throw new Error(
        `Mobile lint flip-back is overdue (deadline ${FLIP_BACK_DEADLINE}, issue #3584). ` +
          `The warn-only sprint window has closed but these are not yet "error":\n  - ${stillRelaxed.join('\n  - ')}\n` +
          'Promote them to "error" (fix the flagged code first — do not weaken the gate), ' +
          'or extend FLIP_BACK_DEADLINE with a justification recorded on issue #3584.'
      );
    }
  });
});
