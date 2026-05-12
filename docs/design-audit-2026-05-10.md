# Reality Portal — design alignment audit (2026-05-10)

Source: `guest-registration-v2-design-system` bundle (Claude Design export).

Compares each `pages/*.html` design to its current `[locale]/<route>` implementation.
Sizes are a coarse signal, not the rubric — the call-out column lists the actual
content gap to focus on. **HIGH** = significant rework needed, **LOW** = cosmetic
gap or already aligned.

## Reality-web pages

| Design page          | Route             | Design KB | Impl KB | Gap     | Notes |
|----------------------|-------------------|-----------|---------|---------|-------|
| **Reality Portal Home** | `/`             | (bundle)  | n/a     | **DONE 2026-05-10** | Added `PopularNeighbourhoods`, `ValueStrip`, `HomeCTA`. Hero + Featured kept (existing implementations are close to design). |
| compare              | `/compare`        |  39       |   1     | **HIGH**  | Implementation is a placeholder. Design has 4-column compare with sticky labels, photo row, 12+ comparable fields. |
| favorites            | `/favorites`      |  40       |   6     | **HIGH**  | Design has filter row (sort + chip filter), count chip, share / export PDF actions, 6-skel loading, empty state. Current is a skeleton route. |
| inquiries            | `/inquiries`      |  40       |  15     | **HIGH**  | Design is two-pane (list left, thread right) ≥1024px. Status pills per inquiry. Current is single-pane list. |
| saved-searches       | `/saved-searches` |  38       |  11     | **HIGH**  | Design has create modal (filter accordion + alert frequency), per-row action set (Run/Edit/Toggle alerts/Delete with confirm), status pill. |
| agency-import        | `/agency/...`     |  66       |  11     | **HIGH**  | XML/CSV import wizard (file drop, mapping table, preview, run). Largest page in the bundle. |
| sell                 | `/sell`           |  47       |  27     | MED     | Design uses 5-step wizard (`listing-edit-step-1..5`). Current is single long form. |
| agency-branding      | `/agency/...`     |  34       |  11     | MED     | Logo upload, color picker, font preview, custom domain. |
| agency-dashboard     | `/agency`         |  30       |  11     | MED     | KPI cards, listings table with status, realtor table, recent inquiries. |
| help                 | `/help`           |  19       |   9     | MED     | Categorised FAQ + contact card. Current is shorter list. |
| journal              | `/journal`        |  18       |  13     | LOW     | Magazine grid + featured article hero. Close to current. |
| profile              | `/profile`        |  19       |  18     | LOW     | Sidebar + sections (account, security, notifications). Close. |
| price-map            | `/price-map`      |  12       |  16     | LOW     | Already at parity (impl actually larger). |
| careers              | `/careers`        |   7       |  10     | LOW     | Mostly content. |
| security             | `/security`       |   6       |   9     | LOW     | 2FA + session list. Close. |
| about                | `/about`          |  12       |   7     | LOW     | Filled out 2026-05-10 (story-content + stats). |
| terms                | `/terms`          |   9       |  10     | LOW     | Aligned. |
| privacy              | `/privacy`        |   8       |  10     | LOW     | Aligned. |
| cookies              | `/cookies`        |   4       |  11     | LOW     | Impl larger (we expanded earlier). |
| contact              | `/contact`        |   8       |   6     | LOW     | Already a structured form; designer tweaks only. |
| report               | `/report`         |   8       |  12     | LOW     | Form aligned. |
| for-agents           | `/for-agents`     |  12       |  12     | LOW     | Aligned. |
| agent-profile        | `/realtor/[id]`   |  30       |  52     | LOW     | Impl larger; visual review. |

## PPT-web pages (design only — no current ppt-web routes audited yet)

| Design page                    | Likely PPT route                | Notes |
|--------------------------------|--------------------------------:|-------|
| ppt-login                      | `/login`                        | Email + password, brand-600. |
| ppt-news                       | `/news`                         | Manager announcements feed. |
| ppt-announcements              | `/announcements`                | Pinned, status pills. |
| ppt-article-detail             | `/news/[id]`                    |        |
| ppt-documents                  | `/documents`                    |        |
| ppt-document-detail            | `/documents/[id]`               |        |
| ppt-upload-document            | `/documents/upload`             | Form + drop zone. |
| ppt-emergency-contacts         | `/emergency`                    |        |
| ppt-disputes                   | `/disputes`                     |        |
| ppt-dispute-detail             | `/disputes/[id]`                |        |
| ppt-file-dispute               | `/disputes/new`                 | Form. |
| ppt-voting                     | `/voting`                       | List. |
| ppt-vote-create                | `/voting/new`                   | Form. |
| ppt-vote-detail                | `/voting/[id]`                  |        |
| ppt-accessibility-settings     | `/settings/accessibility`       | Form. |
| ppt-privacy-settings           | `/settings/privacy`             | Form. |

These are out of scope for this audit pass (separate ppt-web app). Tracked here
for the next round.

## Mobile (`mobile-new-pages.html`, `listing-edit-mobile.html`)

KMP/Compose targets — not actionable from this PR. The bundle uses an Android
device frame for the renders.

## Recommended next-PR order

Once the homepage PR lands, address gaps in this order (effort × visibility):

1. **`/compare`** — implementation is a placeholder, biggest delta, very visible in
   the listings flow.
2. **`/favorites`** — high-traffic post-login surface; current is a stub.
3. **`/saved-searches`** — needed for the alerts feature; design ships the modal.
4. **`/inquiries`** — two-pane needs a layout refactor.
5. **`/sell` → multi-step wizard** — design defines a 5-step flow; aligning will
   simplify validation and reduce the single-page form's complexity.
6. **Agency suite** (`agency-dashboard`, `agency-branding`, `agency-import`) —
   tackle as a single sub-PR; they share table + form patterns.

## Caveats from the bundle README (apply to everything)

1. Inter is a substitution — `next/font/google` already wires this on reality-web.
2. PPT logo is invented — not used on reality-web yet.
3. Lucide icons are a substitution — codebase still uses inline SVGs; the visual
   should match Feather/Lucide stroke style (2px, round caps, 24-unit box).
4. Dark mode tokens are v0 — light surfaces are the canonical pass.
5. Map view in the design is a placeholder; production needs Mapbox/Leaflet.
6. Hero gradient is the only gradient permitted (`#1e40af → #3b82f6`, 135°).
   Already shipped on `HeroSearch`.
