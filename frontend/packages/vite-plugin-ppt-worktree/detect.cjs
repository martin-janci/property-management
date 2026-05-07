// CommonJS shim of detectWorktree for consumers that load via Node directly
// (e.g. Next.js next.config.js). Mirrors the logic in src/index.ts so changes
// must be kept in sync.
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
