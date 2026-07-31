/// <reference types="vite/client" />

/**
 * CSS Modules type declaration.
 *
 * admin-web consumes @ppt/ui-kit source directly (its `main`/`types` point at
 * `src/index.ts`), so tsc compiles ui-kit component files under admin-web's
 * program. Those components import `*.module.css`; ui-kit's own ambient
 * declaration lives outside admin-web's `include`, so declare it here too.
 */
declare module '*.module.css' {
  const classes: { readonly [key: string]: string };
  export default classes;
}
