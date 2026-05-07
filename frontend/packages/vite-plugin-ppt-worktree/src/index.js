import { execSync } from 'node:child_process';
import { resolve } from 'node:path';

/**
 * @typedef {{ name: string, branch: string, isWorktree: boolean }} WorktreeInfo
 */

/**
 * Detect git branch + whether the cwd is in a git worktree.
 * @param {string} [cwd]
 * @returns {WorktreeInfo}
 */
export function detectWorktree(cwd = process.cwd()) {
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

/**
 * @param {string} branch
 * @returns {string}
 */
export function sanitize(branch) {
  return branch
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

/**
 * @typedef {{ cwd?: string }} PluginOptions
 */

/**
 * Vite plugin that injects __PPT_WORKTREE_NAME__ etc. as build-time defines.
 * @param {PluginOptions} [opts]
 * @returns {import('vite').Plugin}
 */
export default function pptWorktreePlugin(opts = {}) {
  return {
    name: 'ppt-worktree',
    config() {
      const info = detectWorktree(opts.cwd);
      return {
        define: {
          __PPT_WORKTREE_NAME__: JSON.stringify(info.name),
          __PPT_WORKTREE_BRANCH__: JSON.stringify(info.branch),
          __PPT_IS_WORKTREE__: JSON.stringify(info.isWorktree),
        },
      };
    },
  };
}
