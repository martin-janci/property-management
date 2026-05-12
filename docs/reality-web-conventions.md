# reality-web — conventions

How to build / extend Reality Portal screens without breaking the design
system or the cross-cutting concerns (dark mode, i18n, SSR). This doc
codifies the lessons from the design-integration sweep — every section
maps to a real bug we hit and fixed.

## 1. Colors must come from tokens

Read every color from a CSS variable (`var(--ppt-*)`). Hardcoded hex
literals like `#fff` or `#1a1d23` will never switch in dark mode.

```diff
- background: #fff;
+ background: var(--ppt-bg-surface);

- color: #6b7280;
+ color: var(--ppt-fg-muted);
```

A CSS-var fallback is fine and recommended: `var(--ppt-bg-surface, #fff)`
gives a sane render if the token is missing at parse time, but the token
still wins when it's defined.

### Badge / pill colors

Use the semantic light/dark pairs, never raw palette values:

```diff
- background: #d1fae5;  /* success-100 */
- color: #065f46;       /* success-800 */
+ background: var(--ppt-color-success-light);
+ color: var(--ppt-color-success-dark);
```

The `*-light` / `*-dark` tokens are paired across themes:
- **Light theme:** light = solid pastel fill, dark = saturated ink.
- **Dark theme:** light = `rgba(accent, 0.18)`, dark = brightened ink (e.g. `#6ee7b7`).

This is the pattern the design system README codified ("dark mode is
paired, never inverted"). Don't introduce new badges that read a raw
`--ppt-color-success` (the un-suffixed token doesn't dim correctly).

### Verifying

```bash
# Run from the repo root. Anything that looks like a hex color in the
# reality-web source should be inside a `var(... fallback)` or inside
# the tokens file.
grep -rn "background:\s*#[0-9a-fA-F]\{3,6\}" frontend/apps/reality-web/src/ \
  | grep -v "var(--"
```

## 2. i18n — every locale must have every key

The app ships in six locales: `sk / cs / de / en / hu / pl`.
Whenever you add a key to one, add it to **all six**. The check:

```bash
# From the repo root
node -e '
  const fs=require("fs"), p="frontend/apps/reality-web/messages";
  const flat=(o,pre="")=>Object.entries(o).reduce((a,[k,v])=>{
    const n=pre?`${pre}.${k}`:k;
    return typeof v==="object"&&v?{...a,...flat(v,n)}:{...a,[n]:v};
  },{});
  const all={};
  for(const f of fs.readdirSync(p)){
    if(!f.endsWith(".json"))continue;
    all[f.replace(".json","")]=flat(JSON.parse(fs.readFileSync(`${p}/${f}`,"utf8")));
  }
  const ref=Object.keys(all.en);
  for(const l of Object.keys(all)){
    const k=Object.keys(all[l]);
    const m=ref.filter(x=>!k.includes(x));
    if(m.length)console.log(l,"missing",m.length,m.slice(0,3));
  }
'
```

If any locale is missing keys, the rendered page falls back to the
default-locale text — confusing on a SK page that suddenly speaks English.

## 3. Dark-mode FOUC — bootstrap is in `<head>`

The color scheme is applied via an inline `<script>` at the very top of
`<head>` in `app/[locale]/layout.tsx`. It MUST stay there. Moving it to
`<body>` (even with Next's `<Script strategy="beforeInteractive">`) breaks
first-paint because App Router queues those scripts after hydration.

The script reads `localStorage["ppt-color-scheme"]` and falls back to
`prefers-color-scheme`. The DevPanel's Theme dropdown reads/writes the
same key — see `@ppt/dev-panel/src/store.ts`.

## 4. Form fields — use the modern primitives

For new forms, import from `@/components/forms`:

| Use case                | Component                 |
|-------------------------|---------------------------|
| Single-line text input  | `FieldInput`              |
| Multi-line text         | `FieldTextarea`           |
| Dropdown                | `FieldSelect`             |
| Section card            | `FormCard`                |
| Sub-section divider     | `FieldGroupTitle`         |
| Sticky bottom bar       | `FormActions`             |

These ship with floating labels, focus rings at the design spec (4 px @
12 % accent), error states, dark-mode pairing, and reduced-motion
fallback. They also already set `suppressHydrationWarning` on inputs,
which silences the autofill-related noise Chrome emits.

Don't reach for a native `<select>` — it will look wildly different on
Windows vs macOS and can't be themed. Use `FieldSelect`.

See `docs/forms-migration-2026-05-10.md` for the per-route migration
schedule.

## 5. CORS — never reach `api.rlt.sk` directly from a worktree

The prod reality-server's CORS allowlist only includes prod hostnames.
Worktree origins (`wt-*.dev.rlt.sk`) get rejected at preflight.

**Always use the same-origin proxy.** `getApiBase()` in
`@/lib/env` returns `''` for worktree hosts, so callers naturally
produce relative URLs like `/api/v1/listings/featured` which the
`next.config.js` rewrite proxies to `api.rlt.sk` server-side. The browser
sees same-origin → no preflight.

```diff
- fetch(`https://api.rlt.sk/api/v1/listings`)
+ fetch(`${getApiBase()}/api/v1/listings`)  // resolves to /api/v1/... on worktrees
```

Dedicated-mode worktrees (`backend: dedicated`) have their own
`api.wt-<name>.dev.rlt.sk` and bypass the rewrite — `getApiBase()`
returns the full URL in that case.

## 6. styled-jsx scoping for `Link` from next-intl

`Link` from `@/i18n/routing` (i.e. next-intl's locale-aware Link) renders
its own `<a>` element. styled-jsx CANNOT add its hashed class to that
`<a>`, which means rules targeting the Link's className silently don't
match.

Wrap Link-targeting selectors in `:global(...)`, and scope them by their
parent element (which DOES carry the hash):

```diff
- <Link className="my-link">…</Link>
- <style jsx>{`
-   .my-link { color: red; }   /* won't apply — Link's <a> has no jsx-hash */
- `}</style>

+ <div className="wrap">
+   <Link className="my-link">…</Link>
+ </div>
+ <style jsx>{`
+   .wrap :global(.my-link) { color: red; }   /* OK */
+ `}</style>
```

This bug bit us four times before we got it codified — anything Link-
rendered (CTAs, nav items, the language switcher's flag rows) needs this
treatment.

## 7. SSR styled-jsx — keep the registry

`app/[locale]/layout.tsx` wraps the providers in `<StyledJsxRegistry>`.
This flushes styled-jsx CSS into the SSR HTML stream; without it every
`<style jsx>` block is injected client-side at hydration, the page
renders unstyled, and the eventual style-application causes a huge CLS
shift (~0.92 measured on the homepage during the integration sweep).

Don't remove it.

## 8. Header / footer / chrome — single source

`<Header />` (`components/ui/Header.tsx`) is the canonical header for
**every** route. Don't duplicate header markup per page (the design
bundle has 22 pages — all with the same header). When the header design
changes, update Header.tsx, not individual pages.

Same for `<Footer />` (`components/ui/Footer.tsx`).

## 9. Token reference (most-used)

```
Background:
  --ppt-bg-app        page background
  --ppt-bg-surface    card / panel background
  --ppt-bg-elevated   popover / modal background
  --ppt-bg-subtle     hover state / muted region

Foreground:
  --ppt-fg-primary    headings, primary copy
  --ppt-fg-secondary  body text
  --ppt-fg-muted      meta text, helper, placeholder
  --ppt-fg-subtle     dividers' labels

Brand & semantic (each pairs across themes):
  --ppt-color-primary               brand-600 (#2563eb light, #3b82f6 dark)
  --ppt-color-primary-soft-bg       brand bg on tiles, hover, badges
  --ppt-color-{success,warning,danger,info}-light  badge fill
  --ppt-color-{success,warning,danger,info}-dark   badge ink

Focus:
  --ppt-focus-ring-shadow          3 px ring at 35 % (default focus)
  + form-specific: 4 px @ 12 % accent on inputs (see forms.css)

Shape:
  --ppt-radius-sm   6 px  (chips, menu items)
  --ppt-radius-md   8 px  (buttons, small inputs)
  --ppt-radius-lg  12 px  (cards, popovers)
  --ppt-radius-xl  16 px  (modals, hero search)

Motion:
  --ppt-transition-fast    120 ms
  --ppt-transition-normal  200 ms
```

## 10. Audit checklist

Before opening a PR that adds/changes a screen:

- [ ] No hardcoded hex colors outside `var(... fallback)` calls
- [ ] Every text string goes through `useTranslations()` (or is a true
      constant like an email address)
- [ ] All six `messages/<locale>.json` files have any new keys you added
- [ ] Page rendered correctly in dark mode (toggle via DevPanel)
- [ ] Form inputs use `FieldInput` / `FieldTextarea` / `FieldSelect` —
      not raw `<input>` / `<select>` / `<textarea>`
- [ ] `Link` from `@/i18n/routing` (not `next/link`)
- [ ] Any new style targeting a Link's className is wrapped in `:global()`
- [ ] No new top-level header / footer markup — extend the shared ones
