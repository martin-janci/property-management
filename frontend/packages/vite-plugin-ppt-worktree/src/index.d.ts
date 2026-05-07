import type { Plugin } from 'vite';

export interface WorktreeInfo {
  name: string;
  branch: string;
  isWorktree: boolean;
}

export function detectWorktree(cwd?: string): WorktreeInfo;

export function sanitize(branch: string): string;

export interface PluginOptions {
  cwd?: string;
}

export default function pptWorktreePlugin(opts?: PluginOptions): Plugin;
