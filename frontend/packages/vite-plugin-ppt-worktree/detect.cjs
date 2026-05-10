// frontend/packages/vite-plugin-ppt-worktree/detect.cjs
//
// CANONICAL implementation of `detectWorktree` and `sanitize`.
//
// This file is the single source of truth for worktree detection logic.
// The ESM entry point (`src/index.js`) re-exports from here so both the Vite
// plugin (ESM) and Next.js's `next.config.js` (CJS via `require`) end up
// running identical code — no parallel copy to keep in sync.
//
// Why CJS as the canonical form: `next.config.js` MUST be CommonJS in Next 14,
// so we'd need a CJS copy regardless. Going the other direction (CJS importing
// ESM) requires top-level await and breaks `next start`. Keeping the canonical
// version in CJS and re-exporting via ESM works in every consumer.
const { execSync } = require('node:child_process');
const { resolve } = require('node:path');

function sanitize(branch) {
  return branch
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function detectWorktree(cwd = process.cwd()) {
  try {
    const branch = execSync('git rev-parse --abbrev-ref HEAD', { cwd, encoding: 'utf8' }).trim();
    const sanitized = sanitize(branch);
    const gitCommonDir = execSync('git rev-parse --git-common-dir', {
      cwd,
      encoding: 'utf8',
    }).trim();
    const gitDir = execSync('git rev-parse --git-dir', { cwd, encoding: 'utf8' }).trim();
    const isWorktree = resolve(cwd, gitDir) !== resolve(cwd, gitCommonDir);
    return { name: sanitized, branch, isWorktree };
  } catch {
    return { name: 'unknown', branch: 'unknown', isWorktree: false };
  }
}

module.exports = { detectWorktree, sanitize };
