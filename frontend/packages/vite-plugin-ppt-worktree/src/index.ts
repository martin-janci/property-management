import { execSync } from 'node:child_process';
import { resolve } from 'node:path';
import type { Plugin } from 'vite';

export interface WorktreeInfo {
  name: string;       // sanitized branch name
  branch: string;     // raw branch
  isWorktree: boolean;
}

export function detectWorktree(cwd: string = process.cwd()): WorktreeInfo {
  try {
    const branch = execSync('git rev-parse --abbrev-ref HEAD', { cwd, encoding: 'utf8' }).trim();
    const sanitized = sanitize(branch);
    const gitCommonDir = execSync('git rev-parse --git-common-dir', { cwd, encoding: 'utf8' }).trim();
    const gitDir = execSync('git rev-parse --git-dir', { cwd, encoding: 'utf8' }).trim();
    const isWorktree = resolve(cwd, gitDir) !== resolve(cwd, gitCommonDir);
    return { name: sanitized, branch, isWorktree };
  } catch {
    return { name: 'unknown', branch: 'unknown', isWorktree: false };
  }
}

export function sanitize(branch: string): string {
  return branch
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export interface PluginOptions {
  cwd?: string;
}

export default function pptWorktreePlugin(opts: PluginOptions = {}): Plugin {
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
