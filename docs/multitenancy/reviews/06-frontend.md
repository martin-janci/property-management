# Review #6 — Frontend (admin-ui, reality-web theming, ppt-web /admin)

**Reviewer:** Code Reviewer #6
**Branch:** `integration/multitenancy-phases-2-5p5`
**Mode:** READ-ONLY (no install, no build, no edits).
**Date:** 2026-05-15

---

## 1. Reality-web tenant theming flow

### Request lifecycle

1. Inbound request hits `frontend/apps/reality-web/src/middleware.ts`. The next-intl middleware does locale routing, then we attach `x-tenant-host` (raw `Host` header) and `x-tenant-pathname` to the response. **No** `/tenant-config` fetch happens at the edge — by design, to avoid doubling the load (`React cache()` memoizes per request).
2. `app/[locale]/layout.tsx` runs in RSC and calls `getTenantConfig()` (`src/lib/tenant-config.ts`).
3. `getTenantConfig()` reads `x-forwarded-host || host` from `headers()` and `fetch`es `${API_INTERNAL_URL}/tenant-config`, **forwarding the `Host` header verbatim** so api-server's host_tenant_middleware resolves the same tenant Caddy did.
4. Response is materialised into a `TenantConfig`, passed to the layout, which:
   - Renders a 503 markup if `feature_flags.building_disabled.enabled`.
   - Otherwise sets `style={brandingStyle}` on `<html>` and inlines `window.__TENANT_CONFIG__ = {tenant_id, feature_flags}` for client-side `useFeatureFlag`.

The api-server `/tenant-config` endpoint (`backend/servers/api-server/src/routes/tenant_config.rs:131`) returns `Cache-Control: public, max-age=60, s-maxage=60` and `Vary: Host`. Platform host (no `ResolvedTenant` extension) returns hard-coded defaults.

### Cache safety

- Next data-cache: `next: { revalidate: 60, tags: [`tenant:${host}`] }` in `tenant-config.ts:124-127`. Tag is host-scoped — `revalidateTag('tenant:acme.example.com')` after a branding flip works.
- **Cache key correctness — ⚠️ leaky.** `fetch(apiUrl, …)` is called with the **same URL** for every host (no host in URL or in a custom cache key). Next 14's data cache keys on URL + headers + method by default; `headers: { Host: host }` does flow into the key in App Router, but this is a fragile contract that future Next minor versions could change. The defensive belt-and-braces fix is to put the host into the URL (e.g. `?host=${encodeURIComponent(host)}`) so the key is unambiguous. As written today, two tenants probably do not collide, but the only thing standing between them and a swapped-brand cache hit is the `headers` -> data-cache-key serializer in Next.
- Page-level ISR (`generateStaticParams`) regenerates per-locale, **not** per-tenant. If `app/[locale]/page.tsx` (or any descendant) is statically rendered, its output is shared across hosts — only the `<html>`'s `style={brandingStyle}` is per-request because the layout is dynamic via `getTenantConfig()` calling `headers()`. This forces dynamic rendering for any subtree that reads tenantConfig, which is correct, but it also means the per-tenant deploy gets zero ISR benefit. Not a bug, but a perf gotcha worth flagging.
- Server vs client: tenant config is awaited in the layout before any markup. There is no flash-of-default-branding because `<html style={…}>` is set in the SSR HTML.

### Branding XSS analysis

`brandingToStyleObject` (`tenant-config.ts:178-205`) is the choke point for tenant-controlled branding:

- Whitelist on var **names**: `^--[a-z0-9-]+$` and length ≤ 64 (`isSafeCssVarName`).
- Reject on var **values**: `<>"\n\r{};` and length > 200 (`sanitizeCssValue`).

Findings:
- `style` is a React `CSSProperties` object, not raw string concatenation, so React handles the value escaping inside the style attribute. The sanitizer is defence-in-depth, not the only line.
- ⚠️ **Sanitizer leaks `url(...)` and CSS expressions.** `--ppt-bg: url("//evil.com/x.png")` passes the `<>"\n\r{};` filter (the value contains `()` and `:` only outside the regex denylist; quotes `"` are denied but `'` is not). `url(…)` in a CSS variable that is later `var(--ppt-bg)`'d into a `background-image` would let an agency's branding row exfiltrate via referrer or load arbitrary assets. Recommend an allowlist: hex/rgb/hsl colour, plain font-family token, no `url(`, no `expression(`, no parentheses at all.
- ⚠️ **Single quote `'` is not in the denylist** despite `"` being denied. CSS attribute strings can use either; closing quote escape is theoretically possible if the value ever flows into a string-quoted context. Low risk inside React's style serialization, still inconsistent.
- ✅ `dangerouslySetInnerHTML` for `__TENANT_CONFIG__` bootstrap takes the JSON-encoded blob (`tenantBootstrap = JSON.stringify(...)`). Provided the source feature_flags / tenant_id can never contain `</script>` (they come from app-controlled flag keys + UUID, so OK), this is safe. There is **no** `JSON.stringify` `</` -> `<\/` escape, however — if any future feature_flag value (DB-controlled JSON Value) contains the literal string `</script>`, it breaks out of the script tag. Recommend `tenantBootstrap.replace(/</g, '\\u003c')` after stringify.
- ✅ `<link rel="icon" href={tenantConfig.branding.logo_url}>` — React escapes attribute values; safe.

### `building_disabled` UX

- Layout returns a self-contained `<html>` with title "Service temporarily unavailable", `<meta name="robots" content="noindex">`, and "{tenantName} is offline for maintenance." Inline-styled, no external CSS.
- ⚠️ **Wrong HTTP status.** This is rendered as the layout output, so the response is **HTTP 200**. Operators / monitoring will see a "good" status code with a 503-ish body; bots may still index it (despite `noindex`); CDN may cache the kill-switch markup as a normal page. Should set the response status via `next/navigation` or by throwing in a `notFound()`-style API. (`notFound()` -> 404; for a true 503 you need a Route Handler / `headers()` call from a server action.)
- ⚠️ Hard-coded English. No i18n on the kill-switch page. Acceptable for an MVP but flag it.
- ✅ The inline-style branch avoids depending on the rest of the design system, which is the correct call when the app is intentionally offline.

---

## 2. admin-ui component inventory

| Component | Props (essentials) | Gotchas |
|---|---|---|
| `<ResourceTable<T>>` | `columns`, `data`, `rowKey`, `actions?`, `emptyMessage?`, `caption?` | Action visibility is **hide-not-disable** (`useCapability` → `null` if not allowed). Per leak #21. ✅ Correct call. ⚠️ Hard-coded class names (`ppt-admin-resource-table`) — no CSS ships with the package, host app must style it. |
| `<SettingsForm<T>>` | `fields` (text/number/boolean/select), `initialValues`, `capability`, `onSubmit`, `header?` | Schema is bespoke, **not** Zod/Yup/JSON Schema. No client-side validation beyond HTML5 `min`/`max`. ⚠️ Submit button text reads "Read-only" when user lacks capability (good — explicit), but the **fields are still rendered** as `disabled` rather than hidden. That's an information disclosure tradeoff (user sees fields exist) — flag, since leak #21 wants the opposite for actions. ⚠️ `Number(e.target.value)` on empty string yields `0`, not `NaN` — silent data corruption when the user clears a number field. |
| `<AuditViewer>` | `entries`, `filter`, `onFilterChange`, `loading?`, `emptyMessage?` | Purely presentational (no fetching). ⚠️ **No pagination** — cursor / offset / page-size props are absent. The component renders every entry passed in. The `audit_log` table is append-only and grows monotonically; the host page must paginate before passing to this component. ⚠️ `<time dateTime={e.created_at}>{e.created_at}</time>` displays the raw ISO string; **not localised**. Per the brief: "date display in user's locale" — fails. ⚠️ `entries.map` uses `e.id` as key but interpolates raw `actor_id` / `target_id` as table cell text — XSS-safe via React, but UUIDs are not human-readable. |
| `<ImpersonationBanner>` | `active`, `targetUserLabel?`, `expiresAt?`, `onEnd?` | ✅ Sticky, `position: sticky; top: 0; zIndex: 9999`, contrast ratio of `#b91c1c` on white is ~5.7:1 (WCAG AA pass). ✅ `role="alert"` + `aria-live="assertive"`. ⚠️ **Not dismissible** (no close button in props) — correct per design intent. ⚠️ `expiresAt` is rendered as `new Date(expiresAt).toLocaleTimeString()` — no live countdown despite the prop comment ("renders a countdown"). It is a one-shot at render. ⚠️ Banner disappears purely on `active=false` — no auto-disappear when `expiresAt` passes; if the parent doesn't unmount it after token expiry, the banner lies. |

### `package.json` correctness

- ✅ `name: "@ppt/admin-ui"`, peer `react ^19.2.0` matches ppt-web's `react ^19.2.0`. No version skew.
- ✅ `private: true`, workspace deps (`@ppt/shared`, `@ppt/ui-kit`) — but **`@ppt/ui-kit` is imported nowhere in admin-ui** (`grep` found no usage). Either drop the dep or actually adopt the design system.
- ✅ `tsconfig.json` has `strict: true`.
- ⚠️ `main: "./src/index.ts"` and `types: "./src/index.ts"` — the package is shipped as **raw TypeScript** sources, no build step in the consumer pipeline. ppt-web (Vite) will transpile on demand, which works, but `dist/` and `outDir` are configured implying a build was intended. Inconsistent. Not blocking.

---

## 3. `useCapability` data source

- **Source of truth:** in-memory React context (`CapabilityProvider`), seeded by the host app at boot.
- **Where the host gets capabilities:** the host is expected to call `GET /api/v1/admin/capabilities/users/:me` (per `index.ts:6` comment) and pass the array as `capabilities` prop. ❌ **No host code does this on this branch.** `AdminRouter` is referenced from `index.ts` but `grep` finds no `<AdminRouter>` mount in `App.tsx` / `main.tsx`. There is no fetcher, no TanStack Query for capabilities, nothing.
- **Refetch policy:** none. Capabilities are static for the React tree's lifetime. If a platform admin's capabilities are revoked mid-session, the UI keeps showing affordances until reload. Backend `require_capability` will still 403, so it's a UX bug not a security bug — but worth noting.
- **N+1 risk:** zero, because there is one upstream fetch by contract. ✅
- ⚠️ **`useCapability` returns `false` when `!isPlatformPrincipal`** — correct intent (a non-platform user should never see admin affordances), but it means the same hook can never be used inside a non-admin tree. Coupling `useCapability` to "admin context" forecloses future use of capability-based UI in normal tenant flows. Minor design smell.
- ⚠️ The capability list (`capabilities.ts`) is hard-coded on the client; backend has a `/admin/capabilities/registry` endpoint as runtime source of truth, but the client never calls it. If backend grows a new capability, client `Capability` type must be republished. Acceptable for closed-enum type safety, but flag the drift risk.

---

## 4. ppt-web `/admin` route gate

### What stops a hand-typed URL?

❌ **Nothing on this branch.** The router gate (`features/admin/router.tsx`) is well-designed:
1. `<RequirePlatformPrincipal>` reads `useCapabilityChecker()` and `<Navigate to="/" replace />` if not platform.
2. Wrapped in `<CapabilityProvider value={…}>`.

But `<AdminRouter>` is **never mounted** in `apps/ppt-web/src/App.tsx`. `grep -rn "AdminRouter"` only matches the feature directory itself. Hand-typing `/admin/agencies` in the browser today either 404s (React Router doesn't know the path) or falls through to whatever wildcard catches it.

When `<AdminRouter>` is eventually mounted:
- `RequirePlatformPrincipal` redirects to `/` rather than showing a "no access" page (correct per leak #21).
- Backend is still the enforcement boundary — the Phase 5 admin-core middleware on `/api/v1/admin/*` 403s any non-platform principal regardless of UI gate.
- ⚠️ **Race:** `isPlatformPrincipal` is a static prop passed at mount. A user whose platform-principal status was revoked mid-session continues to see `/admin` until the next page reload. Backend will 403 every call, so the UI degrades to "everything is empty / failing" rather than redirect — bad UX, no security impact.

### Pages stub status

| Page | Backend wired? |
|---|---|
| `agencies.tsx` | ❌ `data: AgencyRow[] = []`, `console.warn('TODO: suspend')` |
| `users.tsx` | ❌ `data: UserRow[] = []`, `console.warn('TODO: impersonate')` |
| `feature-flags.tsx` | ❌ `console.warn('TODO: PATCH /admin/agencies/:id/feature-flags')` |
| `audit.tsx` | ❌ `entries: AuditEntry[] = []` |
| `platform.tsx` | ❌ `console.warn('TODO: PATCH /admin/platform/settings')` |

All five pages are real components (not literal `// TODO` files) but **none of them call the backend**. The brief asked to verify they were not "TODO stubs"; these are TODO stubs at the data layer. The route surface and capability wiring are real, the API integration is not.

### API path alignment

Comments reference `GET /api/v1/admin/agencies`, `…/users?q=…`, `…/audit?actor_id=…`, `…/capabilities/users/:me`, `PATCH /admin/agencies/:id/feature-flags`, `PATCH /admin/platform/settings`. Phase 5 backend mounts `/admin/*` (per integration plan); these match in shape. ⚠️ Inconsistency: code mostly says `/api/v1/admin/...` but `feature-flags.tsx` and `platform.tsx` say bare `/admin/...`. Pick one.

---

## 5. MFA UX

❌ **Does not exist.** Greps for `mfa|MFA|step.up|recent_mfa` in `frontend/packages/admin-ui/` and `frontend/apps/ppt-web/src/features/admin/` return zero matches.

The only MFA-adjacent code in ppt-web is `features/auth/pages/TwoFactorAuthPage.tsx` (TOTP enable/disable), unrelated.

**What happens when an admin action requires recent MFA and the user has none?**
1. User clicks (e.g.) "Suspend agency" → `onClick` fires.
2. (When wired) the API call is made to `/api/v1/admin/agencies/:id/suspend`.
3. Backend Phase 5 admin-core middleware checks `recent_mfa`, returns 401 / 403 with some MFA-required error code.
4. Frontend has **no axios interceptor**, **no MFA challenge modal**, **no re-login flow**. The error bubbles up as a generic `console.warn` (since handlers are stubs) or, when wired, as whatever the host's TanStack Query default error handling does — a toast at best, silent failure at worst.

This is a meaningful gap. The brainstorming session listed MFA gating as a pillar of the super-admin control plane; the UI has zero affordance for it. Recommend a `<MfaChallengeModal>` component in admin-ui that auth interceptors can trigger on a 401-with-`mfa_required` response.

---

## 6. Cross-cutting

- ✅ TypeScript `strict: true` in `admin-ui/tsconfig.json`.
- ⚠️ `any` count in admin-ui: zero. ✅ Good. But `details: unknown` in `AuditEntry` is never rendered — the audit viewer hides the most useful field.
- ⚠️ **i18n: English-only.** `<RequirePlatformPrincipal>`, every page heading, every empty message, the impersonation banner, the kill-switch page — all hard-coded English. Acceptable for an MVP super-admin tool, but every other ppt-web feature uses i18next. Inconsistent.
- ⚠️ **No accessibility test coverage.** Admin pages have no `*.a11y.test.tsx` despite ppt-web having an axe-core a11y harness. Forms do use `<label htmlFor>` correctly. The `caption?` prop on `ResourceTable` is optional and unused by the pages — set it for screen-reader users.
- ⚠️ **No `ImpersonationBanner` mount site.** The component exists in admin-ui but nothing in ppt-web imports it. An impersonation flow that hides itself when the banner isn't rendered is exactly what leak #21 was trying to prevent.

---

## 7. Verdict — Top 5 Issues

| # | Severity | Issue |
|---|---|---|
| 1 | ❌ | `AdminRouter` is **not mounted** in ppt-web. `/admin/*` is dead code on this branch. URL gate, capability provider, all five pages, the impersonation banner — all unreachable. |
| 2 | ❌ | **MFA UX is missing.** No challenge modal, no 401 interceptor, no re-auth flow. Admin actions requiring recent MFA will silently fail. |
| 3 | ⚠️ | **Branding sanitizer is too permissive.** `url(…)` and `'` are not denied; a malicious agency could inject `--ppt-bg: url(//evil.com)` via `css_vars` if any consumer ever uses a tenant-controlled var as a `background-image`. |
| 4 | ⚠️ | **Kill-switch page returns HTTP 200**, not 503, and is English-only. CDN/monitoring/SEO will treat the maintenance page as normal content. |
| 5 | ⚠️ | **All five admin pages are data-layer stubs.** Real components, real routes, real capability wiring — but every page renders an empty array and every action is `console.warn('TODO')`. Phase 5 ships a UI shell, not a functioning admin console. |

### Other notable findings

- ⚠️ Audit viewer has no pagination and renders ISO strings without locale formatting.
- ⚠️ `ImpersonationBanner.expiresAt` doesn't actually count down despite the doc comment.
- ⚠️ `SettingsForm` numeric `Number('')` → `0` silent data corruption.
- ⚠️ `useCapability` returning `false` for non-platform principals couples the hook to admin context; not reusable elsewhere.
- ⚠️ `tenantBootstrap` JSON injection is safe today but lacks `</` -> `<\/` escape; a future feature_flag value could break out of the inline `<script>`.
- ⚠️ Capability list is duplicated client-side; `/admin/capabilities/registry` is the SoT but never consulted.
- ⚠️ `@ppt/ui-kit` is a declared dep in `admin-ui/package.json` but unused.
- ⚠️ Admin section is English-only; rest of ppt-web is i18next.

### Summary

| Area | Status |
|---|---|
| Reality-web theming flow correctness | ✅ Mostly sound (cache key relies on Next behaviour; document or harden) |
| Branding XSS sanitization | ⚠️ Defensible but leaky on `url(…)` and `'` |
| `building_disabled` UX | ⚠️ Wrong status code, no i18n |
| admin-ui component contracts | ⚠️ Reasonable shells; gaps in audit/impersonation |
| `useCapability` data source | ⚠️ Right shape, no fetcher exists in host |
| ppt-web `/admin` route gate | ❌ Gate logic correct, **never mounted** |
| MFA UX | ❌ Absent |
| TS strictness / i18n / a11y | ⚠️ Strict OK; i18n + a11y missing in admin section |

**Overall:** ⚠️ — Phase 3 theming is essentially production-ready with two tightening recommendations (sanitizer, 503 status). Phase 5's `/admin` is a scaffold: well-architected components and a correct router gate, but unmounted, unwired to the backend, and missing the MFA challenge UX that the security model assumes exists.
