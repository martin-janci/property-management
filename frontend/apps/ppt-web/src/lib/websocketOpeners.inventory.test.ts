/// <reference types="vitest/globals" />
/**
 * WebSocket-opener inventory guard (credential-hygiene, follow-up to #3696).
 *
 * #3696 moved the shared notification socket off the leaking
 * `new WebSocket('…/ws?token=<jwt>')` transport and onto the
 * `Sec-WebSocket-Protocol` handshake header (a query-string JWT leaks the
 * bearer credential into api-server access logs, reverse-proxy logs, and the
 * browser's own history — a single captured line is enough for session
 * takeover). That fix landed on ONE opener (`lib/websocket.ts`). The IoT
 * sensor socket (`features/iot/hooks/useIotWebSocket.ts`) was a second opener
 * that had the same leak and was fixed separately.
 *
 * The per-opener regression tests (`lib/websocket.test.ts`,
 * `features/iot/hooks/useIotWebSocket.tokenTransport.test.tsx`) each prove
 * their own socket never puts the token in the URL. What neither can prove is
 * the *coverage* claim this task exists to confirm: that the strip reaches
 * EVERY WebSocket opener on ppt-web and no third opener has silently appeared
 * without a no-token-in-URL assertion.
 *
 * This guard closes that gap structurally:
 *   1. It scans the real source tree for every `new WebSocket(` call site and
 *      asserts the set exactly matches {@link EXPECTED_OPENERS}. A future
 *      opener added anywhere under src/ fails this test until it is registered
 *      here — which is the explicit, reviewed act of acknowledging a new
 *      credential-bearing socket.
 *   2. For each registered opener it asserts a credential-hygiene regression
 *      test exists AND carries a negative `token=`-in-URL assertion, so every
 *      opener is provably covered by the #3696 invariant, not just the
 *      notification socket.
 *
 * Net effect: "the token-in-URL strip covers every WebSocket opener" becomes a
 * continuously enforced invariant instead of a point-in-time manual check.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

// Vitest runs with cwd set to this package root (apps/ppt-web).
const srcDir = join(process.cwd(), 'src');

/**
 * Every production WebSocket opener on ppt-web, each paired with the
 * regression test that proves it never puts the JWT in the connection URL.
 * Paths are relative to `src/`.
 */
const EXPECTED_OPENERS: ReadonlyArray<{
  /** Source file containing the `new WebSocket(` call. */
  source: string;
  /** Test asserting this opener keeps the token out of the URL. */
  hygieneTest: string;
}> = [
  {
    source: 'lib/websocket.ts',
    hygieneTest: 'lib/websocket.test.ts',
  },
  {
    source: 'features/iot/hooks/useIotWebSocket.ts',
    hygieneTest: 'features/iot/hooks/useIotWebSocket.tokenTransport.test.tsx',
  },
];

/** True for test/spec sources that are not production openers. */
function isTestFile(relPath: string): boolean {
  return /\.(test|spec|a11y\.test)\.tsx?$/.test(relPath) || relPath.endsWith('.d.ts');
}

/** Recursively collect production `.ts`/`.tsx` sources under src/ (POSIX-relative). */
function productionSources(): string[] {
  const out: string[] = [];
  function walk(absDir: string, relDir: string): void {
    for (const entry of readdirSync(absDir, { withFileTypes: true })) {
      const rel = relDir ? `${relDir}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        walk(join(absDir, entry.name), rel);
      } else if (/\.tsx?$/.test(entry.name) && !isTestFile(rel)) {
        out.push(rel);
      }
    }
  }
  walk(srcDir, '');
  return out;
}

/** Read a source file relative to src/, returning '' if it does not exist. */
function readSrc(relPath: string): string {
  try {
    return readFileSync(join(srcDir, relPath), 'utf8');
  } catch {
    return '';
  }
}

describe('WebSocket opener inventory (credential hygiene — follow-up to #3696)', () => {
  it('every production `new WebSocket(` call site is a registered, token-stripped opener', () => {
    const actualOpeners = productionSources()
      .filter((rel) => /\bnew WebSocket\s*\(/.test(readSrc(rel)))
      .sort();
    const expectedOpeners = EXPECTED_OPENERS.map((o) => o.source).sort();

    // Set-equality is the tripwire: a NEW opener fails this until it is added
    // to EXPECTED_OPENERS (with a hygiene test), and a stale registry entry
    // that no longer opens a socket fails it too.
    expect(
      actualOpeners,
      'A `new WebSocket(` call site is missing from — or stale in — EXPECTED_OPENERS. ' +
        'Every WebSocket opener must carry the #3696 token-in-URL strip (JWT in the ' +
        '`Sec-WebSocket-Protocol` handshake, never the query string). Register the new ' +
        'opener here and add a no-token-in-URL regression test for it.'
    ).toEqual(expectedOpeners);
  });

  it.each(EXPECTED_OPENERS.map((o) => [o.source, o.hygieneTest]))(
    'opener "%s" is covered by a no-token-in-URL regression test (%s)',
    (source, hygieneTest) => {
      // The registered source must actually be an opener...
      expect(
        /\bnew WebSocket\s*\(/.test(readSrc(source)),
        `${source} is registered as a WebSocket opener but contains no \`new WebSocket(\` call.`
      ).toBe(true);

      // ...and its hygiene test must assert the token never lands in the URL.
      const testSource = readSrc(hygieneTest);
      expect(testSource, `Hygiene test ${hygieneTest} for opener ${source} is missing.`).not.toBe(
        ''
      );
      // Matches both `.not.toContain('token=')` and `.not.toMatch(/[?&]token=/)`.
      expect(
        /\.not\.to(Contain|Match)\([^)]*token=/.test(testSource),
        `${hygieneTest} must assert the opener's connection URL never contains \`token=\` ` +
          '(the leaking transport #3696 removed).'
      ).toBe(true);
    }
  );
});
