/**
 * Client-safe CSS sanitization (Phase 3: Hosting & Theming).
 *
 * Extracted from `tenant-config.ts` so that client components can import
 * `sanitizeCssValue` without dragging in the server-only module (which pulls
 * `next/headers` and React's `cache`, both of which break the client bundle).
 *
 * This file MUST stay free of server-only imports (`next/headers`, `react`
 * `cache`, etc.) — it is pulled into both the server and client bundles.
 */

export function isSafeCssVarName(name: string): boolean {
  return /^--[a-z0-9-]+$/.test(name) && name.length <= 64;
}

/**
 * Sanitize a CSS custom-property value before it lands in inline `style=`.
 *
 * Defense layers (N7):
 * - Reject characters that break out of the style attribute or chain a
 *   second declaration: `<`, `>`, `"`, newlines, `{`, `}`, `;`.
 * - Length cap (200 chars) — bigger than any legit color / font value.
 * - Denylist tokens that resolve to a network fetch or arbitrary script
 *   when the var ends up in a property that evaluates URLs (background-image,
 *   list-style-image, mask, cursor, etc.): `url(`, `expression(`,
 *   `javascript:`, `data:`. Logo URLs flow through `branding.logo_url`
 *   (a typed string the layout renders into a real `<link rel="icon">`),
 *   never through `css_vars`.
 * - Positive allowlist for color-shaped vars: when the var name suggests
 *   a color (matches `/color|background|border/`), the value MUST look
 *   like a CSS color literal (hex, rgb/rgba, hsl/hsla, or a bare CSS
 *   color keyword). Anything else (e.g. `var(--evil)`, `inherit`,
 *   `attr(...)`) is rejected.
 *
 * Returns `null` when the value cannot be made safe — callers drop the
 * declaration rather than emit a partial / suspicious value.
 */
export function sanitizeCssValue(value: string, varName?: string): string | null {
  if (typeof value !== 'string') return null;
  if (value.length > 200) return null;
  // Structural breakouts.
  if (/[<>"\n\r{};]/.test(value)) return null;

  // Token denylist (case-insensitive). `data:` is denied wholesale even
  // though `data:image/...` is theoretically harmless — branding rows do
  // not need it (logos go through `logo_url`), and allow-listing the
  // `image/png` subset would reopen the door to `data:text/html` via
  // case / whitespace tricks.
  const lower = value.toLowerCase().replace(/\s+/g, '');
  if (
    lower.includes('url(') ||
    lower.includes('expression(') ||
    lower.includes('javascript:') ||
    lower.includes('data:')
  ) {
    return null;
  }

  // Color-shaped var: enforce a CSS color literal. The allowed shapes are
  // intentionally narrow — no `var(...)` indirection (would let a tenant
  // chain to an unsanitized var), no `currentColor` / `inherit` (low
  // value, easy to bypass intent), no functional notations beyond rgb/hsl
  // families.
  if (varName && /color|background|border/i.test(varName)) {
    const trimmed = value.trim();
    const isColorLiteral =
      /^#[0-9a-f]{3,8}$/i.test(trimmed) ||
      /^rgba?\([^()]*\)$/i.test(trimmed) ||
      /^hsla?\([^()]*\)$/i.test(trimmed) ||
      /^[a-z]+$/i.test(trimmed); // bare keyword: red, transparent, etc.
    if (!isColorLiteral) return null;
  }

  return value;
}
