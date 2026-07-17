import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { sitemap } from '../data';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/** Absolute path of the committed sitemap.json artifact. */
export const SITEMAP_JSON_PATH = path.join(__dirname, 'sitemap.json');

/**
 * Build the sitemap.json payload for non-TypeScript consumers (e.g., Rust
 * backend tests) from `../data`.
 *
 * Pure: no wall-clock timestamp, no filesystem access — the same `../data`
 * input always yields the same object. This determinism is load-bearing:
 * the file is committed and embedded into the Rust backend via `include_str!`,
 * so a `new Date()` value made every regeneration produce a spurious diff
 * (churn hotspot). See PR #2397.
 */
export function buildSitemapJson() {
  return {
    routes: {
      'ppt-web': sitemap.routes['ppt-web'],
      'reality-web': sitemap.routes['reality-web'],
    },
    screens: {
      mobile: sitemap.screens.mobile,
    },
    endpoints: {
      'api-server': sitemap.endpoints['api-server'],
      'reality-server': sitemap.endpoints['reality-server'],
    },
    flows: sitemap.flows,
    metadata: {
      version: '1.0.0',
      stats: {
        ppt_web_routes: sitemap.routes['ppt-web'].length,
        reality_web_routes: sitemap.routes['reality-web'].length,
        mobile_screens: sitemap.screens.mobile.length,
        api_server_endpoints: sitemap.endpoints['api-server'].length,
        reality_server_endpoints: sitemap.endpoints['reality-server'].length,
        flows: sitemap.flows.length,
      },
    },
  };
}

/**
 * Serialize the payload exactly as it is committed to disk: 2-space indent plus
 * a trailing newline (keeps the artifact POSIX-clean). This is the single
 * source of truth for the on-disk bytes — both the generator below and the
 * drift-guard test (`tests/sitemap.test.ts`) call it, so they agree
 * byte-for-byte.
 */
export function serializeSitemapJson(): string {
  return `${JSON.stringify(buildSitemapJson(), null, 2)}\n`;
}

function writeSitemapJson(): void {
  fs.writeFileSync(SITEMAP_JSON_PATH, serializeSitemapJson());
  console.log(`Generated sitemap.json at ${SITEMAP_JSON_PATH}`);
  console.log('Stats:', buildSitemapJson().metadata.stats);
}

// Only write when executed directly (`tsx src/json/generate.ts`), not when the
// pure builders above are imported by the drift-guard test.
const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : '';
if (invokedPath === __filename) {
  writeSitemapJson();
}
