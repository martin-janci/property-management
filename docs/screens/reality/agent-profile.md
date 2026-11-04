---
id: reality/agent-profile
name: Agent Profile (public)
product: reality
implementations:
  reality-web:
    component: AgentProfilePage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/listing-detail
    rel: parent
  - id: reality/agency-dashboard
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - listing-card
  - rating-stars
  - kv-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/agent-profile.html
    frame: default-with-listings-and-reviews / empty-no-listings / loading / error
useCases:
  - UC-49
  - UC-51
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->
<!-- Reconciled 2026-11-04 against the shipped page.tsx after PR #3225 wired
     it to real data. Items the original design envisioned but that have no
     backing reality-server endpoint are tagged [-] with a reason. -->

### Hero
- [x] [w] Gradient cover band + 96px circular avatar (🏠 placeholder when no photo) + H1 + verified badge ("✓ Overený maklér", shown when `license_verified_at` set) + experience-years line
- [x] [w] "Kontaktovať" primary button (opens the contact modal)
- [-] [w] ~~"Sledovať" ghost (notify on new listings)~~ — no agent-follow endpoint exists; dropped in PR #3225
- [-] [w] ~~Agent name / role / agency link~~ — the public profile contract carries no display name, so the H1 falls back to `tagline` → "Realitný maklér"; a name is never fabricated

### Stats band
- [x] [w] Aktívne inzeráty (`active_listings`) · Inzeráty celkom (`total_listings`) · Zobrazenia (`total_views`) · Hodnotenie (avg ★ + count, only when reviews exist)
- [-] [w] ~~Predané za 12m · Avg odpoveď · Na portáli (rokov)~~ — no backing metrics in the profile contract; replaced by the real fields above

### Section · O makléri (bio)
- [x] [w] Bio paragraph (`profile.bio`), rendered only when present

### Section · Hodnotenia
- [x] [w] H2 with live review count; empty state ("Zatiaľ žiadne hodnotenia.") when none
- [x] [w] Vertical list of review cards (reviewer name + 5-star rating + date + body + "✓ Overené" verified-buyer pill)
- [-] [w] ~~"Všetkých 47 →" link~~ — reviews are fetched as a single page (`limit`-bounded); no dedicated reviews-list route

### Section · Inzeráty tohto makléra
- [-] [w] ~~H2 + "Všetkých 14 →" link + 3-up listing-card grid~~ — no realtor-scoped public listings endpoint exists; the whole listings section is omitted (PR #3225)

### Sidebar · Kontaktné údaje
- [x] [w] Phone (tel:) + public email (mailto:) when the agent opted in (`show_phone`/`show_email`); "nezverejnil kontaktné údaje" fallback otherwise
- [x] [w] License number, member-since, specializations (pills), languages
- [x] [w] "Odoslať správu" button (opens the contact modal)

### Contact modal
- [x] [w] mailto handoff with an optional message body when email is public; tel fallback when only phone is public; no server submit (no realtor-direct inquiry endpoint — inquiries are listing-scoped)

### Footer
- [x] [w] Standard portal footer

## States

- **Default**: profile + reviews fetched from the real endpoints; verified badge when `license_verified_at` set
- **Empty**: "Zatiaľ žiadne hodnotenia." under the reviews H2; contact sidebar shows "nezverejnil kontaktné údaje" when no channels are public
- **Loading**: "Načítavam…" placeholder between Header and Footer while the profile query is in flight
- **Not found / Error**: "Maklér sa nenašiel" panel (header+footer retained) on a 404 or any fetch error

## Notes

### Broader context

UC-49 + UC-51 public-facing agent profile. Linked from every listing-detail (agent card click). Drives trust + conversion via verified badge + reviews. Read-only on public side; agents edit their profile in `reality/agency-dashboard` agent management.

### Specific (recent)

- **Wired to real data (PR #3225).** Page fetches `GET /api/v1/realtors/{id}/profile` and `GET /api/v1/realtors/{id}/reviews` client-side via TanStack Query (mirrors the `agency/[slug]` public-profile pattern); the previous hardcoded `MOCK_AGENT` and its faked contact submit are gone. Code: `frontend/apps/reality-web/src/app/[locale]/realtor/[id]/page.tsx`; API helpers in `src/lib/realtor-api.ts` (`getRealtorPublicProfile`, `getRealtorReviews`).
- **Endpoints not yet in `@ppt/sitemap`.** The two realtor endpoints above are live in reality-server but are NOT registered in `frontend/packages/sitemap/src/data/reality-server/index.ts`, so `endpoints:` is kept empty (adding unknown ids fails `/screens validate`). Registering them in the sitemap package is a separate, sitemap-owner change — track as follow-up drift.
- **No display name in the public contract** — the H1 falls back to `tagline` → "Realitný maklér"; names are never fabricated.
- **No agent-follow and no realtor-direct inquiry endpoint** — the "Sledovať" button and a server-side contact submit were dropped; the modal hands off to the agent's real channels (mailto/tel). Inquiries remain listing-scoped (`POST /api/v1/inquiries/contact/{listing_id}`).
- **No realtor-scoped public listings endpoint** — the "Inzeráty tohto makléra" grid from the design is omitted entirely.
- Reviews are post-transaction only — buyer must have actually inquired and the agent must confirm meeting/transaction. Prevents review bombing.
- Verified badge ties to UC-23 ID-verification; high-trust signal.

## Agent Log

<!-- newest entries on top -->

- 2026-11-04 — agent: reconciled after PR #3225 rewrite — buildStatus in-progress→shipped; checklist/states/notes rewritten to match the real-data page (profile+reviews via TanStack Query, honest mailto/tel contact, no mock); flagged dropped design items (follow, sold/avg-response stats, listings grid, reviews-list link) as no-backing-endpoint; noted realtor profile/reviews endpoints absent from @ppt/sitemap so `endpoints:` stays empty.
- 2026-05-09 — agent: bootstrapped from bundle (pages/agent-profile.html — 4 states: default / empty / loading / error); UC-49/51; parent listing-detail
