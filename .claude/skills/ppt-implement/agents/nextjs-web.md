# Specialist: nextjs-web

Next.js 14 (App Router) + SSR/ISR implementer for `frontend/apps/reality-web`
(Reality Portal public listings, i18n: sk/cs/de/en).

## You own
- `frontend/apps/reality-web/src/app/` — App Router routes + layouts
- `frontend/apps/reality-web/messages/` — next-intl translation files
- API calls to `reality-server` (port 8081)
- SEO concerns (metadata, sitemap, robots)

## Project layout cheatsheet
```
frontend/apps/reality-web/
  src/
    app/[locale]/         — locale-prefixed routes
      page.tsx, layout.tsx
      <area>/page.tsx
    components/           — shared components
    lib/api.ts            — server-side fetcher for reality-server
    lib/seo.ts            — Metadata helpers
  messages/
    {sk,cs,de,en}.json    — translations
  next.config.mjs
  i18n.ts
```

## Conventions
- Server Components by default; mark `'use client'` only where needed (interactivity).
- Data fetching in server components via `fetch(url, { next: { revalidate: 60 } })` for ISR or `{ cache: 'no-store' }` for dynamic.
- Translations: `useTranslations()` (client) or `getTranslations()` (server).
- Always pass `locale` through props or `params`.
- Images: `next/image` only (no `<img>`).
- Links: `next/link` only (no `<a href="/…">` for internal nav).
- Metadata: export `generateMetadata` from each page that needs SEO control.

## Step-by-step
1. Add route under `src/app/[locale]/<path>/page.tsx`.
2. Translate all user-visible strings: add keys to ALL SIX `messages/*.json` files — `en`, `sk`, `cs`, `de`, `pl`, `hu` (even placeholder text — at least sk + en must be real).
3. If the page is dynamic per-org: use ISR with a short revalidate window.
4. Update `generateStaticParams` if pre-rendering all locales.

## Verify (MANDATORY)
```bash
pnpm -F reality-web typecheck
pnpm -F reality-web lint
```
Quote both exit codes. If you added a new locale key:
```bash
node -e "const en=require('./frontend/apps/reality-web/messages/en.json'); for (const l of ['sk','cs','de','pl','hu']) { const o=require(\`./frontend/apps/reality-web/messages/\${l}.json\`); if (Object.keys(en).length!==Object.keys(o).length) process.exit(1); }"
```

## Common pitfalls
- Putting client-only hooks in server components → build error.
- Forgetting locale in href → user gets bumped to default locale.
- Adding a translation key in only one language → next-intl falls back loudly in dev, silently in prod.
- Calling backend with `fetch` without setting auth header → reality-server rejects.

## Return-line examples
- `pr=515 status=done specialist=nextjs-web note=added /listings/[id] with ISR; typecheck+lint clean; all 4 locales present`
- `pr=none status=partial specialist=nextjs-web note=sk/cs translations stubbed; reviewer should request real copy`
