/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string;
  readonly VITE_WS_URL: string;
  readonly VITE_API_DEFAULT?: string;
  /** Comma-separated exact origins allowed to activate layout preview mode.
   *  Unset/empty = preview never activates (deny by default). */
  readonly VITE_LAYOUT_PREVIEW_PARENT_ORIGINS?: string;
  readonly DEV: boolean;
  readonly PROD: boolean;
  readonly MODE: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

// Injected by @ppt/vite-plugin-worktree
declare const __PPT_WORKTREE_NAME__: string;
declare const __PPT_WORKTREE_BRANCH__: string;
declare const __PPT_IS_WORKTREE__: boolean;
