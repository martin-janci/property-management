---
id: ppt/error-handling-toasts
name: Error Handling & Toast Notifications
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    component: ToastProvider
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: n/a
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/server-error
    rel: sibling
  - id: ppt/session-expired
    rel: sibling
  - id: ppt/forbidden
    rel: sibling
sharedComponents:
  - toast
  - banner
  - error-boundary
  - offline-indicator
diagrams: []
useCases: []
epics: []
owner: pm-frontend
---

# Error Handling & Toast Notifications

Cross-cutting feedback layer for ppt-web. Not a routed screen — it is the set of
app-shell primitives that surface success / failure / connectivity feedback on
top of every other screen. Captured as a screen-map so the orphan capability
("Error Handling and Toast Notifications") is referenceable from the screen tree
instead of being undocumented.

The capability spans four shipped pieces:

1. **Toast notifications** — `ToastProvider` + `useToast` (`components/Toast.tsx`),
   mounted high in the app tree (`App.tsx`).
2. **Global error boundary** — `ErrorBoundary` (`components/ErrorBoundary.tsx`),
   mounted at the root in `main.tsx`, with an i18n fallback UI + retry/reload.
3. **Offline / reconnection indicator** — `OfflineIndicator` +
   `useNetworkStatus` (`components/OfflineIndicator.tsx`, `hooks/useNetworkStatus.ts`).
4. **API error parsing** — `parseApiError` / `formatValidationErrors` /
   `getFieldError` (`lib/errorHandler.ts`), which normalise backend error
   payloads into a `ParsedApiError` that feature pages feed into `showToast`.

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Toast system
- [x] [w] `ToastProvider` exposes `showToast` / `removeToast` via `useToast`; throws if used outside the provider
- [x] [w] Four toast types: `success` / `error` / `info` / `warning`, each with its own icon + styling
- [x] [w] Title + optional message; optional single action button (`{ label, onClick }`)
- [x] [w] Type-defaulted auto-dismiss durations: success/info 5s, warning 7s, error persistent (`duration = 0`); per-toast `duration` override honoured
- [x] [w] Max 3 visible toasts at once (`MAX_VISIBLE_TOASTS`); only the most-recent 3 render, older ones still auto-dismiss
- [x] [w] Copy-to-clipboard button on toasts (clipboard API with `execCommand` fallback)
- [x] [w] Timeouts tracked per-toast and cleared on manual dismiss + on provider unmount (no leaked timers)
- [x] [w] a11y: container `role="region"` + `aria-label`; each toast `role="alert"`; `aria-live="assertive"` for errors, `"polite"` otherwise

### Global error boundary
- [x] [w] `ErrorBoundary` catches render / lifecycle / constructor errors anywhere in the tree
- [x] [w] Friendly fallback UI with "try again" + "reload page", i18n strings
- [x] [w] Optional `onError` reporting callback (Sentry-style) + optional custom `fallback`
- [x] [w] Mounted at app root in `main.tsx`

### Offline / connectivity
- [x] [w] `OfflineIndicator` banner shows on offline, "reconnected" message on recovery; `role="alert"` + `aria-live="assertive"`
- [x] [w] `useNetworkStatus` via `useSyncExternalStore` on browser `online`/`offline` events; exposes `isOnline`, `wasOffline`, `lastOnlineAt`, `lastOfflineAt`
- [x] [w] Mounted in `App.tsx` shell above the routed content

### API error normalisation
- [x] [w] `parseApiError` returns `ParsedApiError` (title, message, code, requestId, fieldErrors, status, isNetworkError, isRateLimit, retryAfter)
- [x] [w] `formatValidationErrors` + `getFieldError` for field-level form errors
- [x] [w] Feature pages wire mutation/query failures into `showToast({ type: 'error', ... })`

## States

- **Idle**: no toasts; no offline banner; routed content rendered normally.
- **Success toast**: brief confirmation slides in (success icon), auto-dismisses after 5s.
- **Error toast**: persistent error toast (`duration = 0`) with copy button; user must dismiss.
- **Validation error**: feature form maps `fieldErrors` to inline messages; a summary toast may also fire.
- **Offline**: `OfflineIndicator` banner pinned; mutations that fail surface a network-error toast/banner.
- **Reconnected**: transient "back online" message via `wasOffline`.
- **Render crash**: `ErrorBoundary` fallback replaces the subtree with retry/reload.

## Notes

### Broader context

This is the shared feedback substrate every feature page depends on. It is
intentionally provider/hook-shaped rather than route-shaped — there is no URL
for it. The `server-error` / `session-expired` / `forbidden` screens are the
*routed* error destinations; this screen-map covers the *in-app, non-routed*
feedback primitives (toasts, the global boundary, the offline banner) that sit
underneath them.

### Specific (recent)

- 2026-06-28 — coverage-gap verify (task 79-3): implementation confirmed fully
  shipped on `dev` — `ToastProvider`/`useToast`, `ErrorBoundary`, `OfflineIndicator`/
  `useNetworkStatus`, and `lib/errorHandler.parseApiError` all present and wired
  (`App.tsx` mounts ToastProvider + OfflineIndicator; `main.tsx` mounts ErrorBoundary).
  Backed by `Toast.test.tsx` + `Toast.a11y.test.tsx`. Only gap was the missing
  screen-map (orphan epic) — created here. No code change required.
- Error toasts are persistent by design (`ERROR_DURATION = 0`) so users acknowledge
  failures; success/info default to 5s, warning to 7s.
- Toast copy-to-clipboard uses `navigator.clipboard` with a `document.execCommand`
  fallback — keep both paths when touching `ToastItem`.
- `useNetworkStatus` is `useSyncExternalStore`-based; do not convert it to
  `useEffect`+`useState` (avoids tearing on concurrent renders).
- `parseApiError` is the single normalisation point for backend error envelopes —
  new feature pages should route failures through it rather than reading
  `error.response` ad-hoc.

## Agent Log

<!-- newest entries on top -->

- 2026-06-28 — agent: created screen-map for the orphan "Error Handling & Toast Notifications" capability; verified ppt-web implementation shipped (Toast/ErrorBoundary/OfflineIndicator/errorHandler) and wired in App.tsx + main.tsx; checklist marked shipped; sitemapRefs left `{}` (non-routed cross-cutting capability); related to server-error/session-expired/forbidden.
