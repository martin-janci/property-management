/**
 * CSS Modules type declaration.
 *
 * Tells TypeScript that any *.module.css import returns
 * a plain object of string keys (class names).
 */
declare module '*.module.css' {
  const classes: { readonly [key: string]: string };
  export default classes;
}
