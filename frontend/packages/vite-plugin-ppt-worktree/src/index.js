// frontend/packages/vite-plugin-ppt-worktree/src/index.js
//
// ESM entry point for Vite consumers. The Next.js consumer (which uses
// CommonJS in `next.config.js`) imports from `../detect.cjs` directly via the
// package's `./detect` export. Both entry points share **one** implementation
// of `detectWorktree` / `sanitize` — this file re-exports them from
// `detect.cjs` rather than maintaining a parallel copy. That guarantees the
// ESM and CJS consumers can never silently drift.
import detect from '../detect.cjs';

const { detectWorktree, sanitize } = detect;

export { detectWorktree, sanitize };

/**
 * @typedef {{ name: string, branch: string, isWorktree: boolean }} WorktreeInfo
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
