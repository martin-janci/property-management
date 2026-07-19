# Layout & Content Manager — System Design

**Date:** 2026-07-19
**Status:** Brainstorm / design proposal (not yet scheduled)
**Scope:** All four apps (ppt-web, reality-web, accounting-web, mobile RN) + KMP mobile-native, controlled from admin-web

## 1. Problem statement

We want a universal framework to control **which components appear on which
pages, in what order, and in what display mode** — across the Reality portal,
the Property Management app, the Accounting app, and their mobile
counterparts — from a single superadmin surface in admin-web.

Requirements distilled:

- Show/hide natural page components (landing-page sections, listing-detail
  blocks, list-page widgets) without a code deploy.
- Reorder sections and switch **display modes** (e.g. list vs grid vs map),
  where the set of valid modes is defined by the frontend component itself.
- **Never break a page.** Hiding an optional component collapses cleanly;
  hiding/removing a mandatory component renders a placeholder instead.
- Per-platform customization (web vs mobile) without forking page configs.
- Superadmin editor with live preview, draft → publish workflow, versions,
  rollback, audit.

## 2. Prior art & the core lesson

This problem class is **server-driven UI (SDUI) with a CMS-style control
plane**. The industry record is unusually clear about what works and what
fails:

| Worked | Failed |
|---|---|
| Airbnb Ghost Platform — Screens → Sections, semantic components, layouts per form factor | Spotify HubFramework — generic `Row`/`Column`/`Text` primitives → "debugging archeology", deprecated |
| Yelp CHAOS — server filters payload by client-declared capabilities; per-feature error isolation | Uber Screenflow — company-wide generic engine, never finished |
| Duolingo — UI config and data versioned independently; stale clients render cached layout + fresh data | |
| Shopify Shop app — typed section subtypes, per-section data loaders | |
| DoorDash — GUI composer (Mosaic) built only *after* a year of contract hardening | |

**The core lesson:** do not build a generic layout engine. Build a **section
visibility & configuration system over semantic components we already own**.
The server decides *which* named sections render, in *what order*, in *which
mode*, with *what props* — every section remains a real, hand-written
component in each frontend.

Sources: [Airbnb](https://medium.com/airbnb-engineering/a-deep-dive-into-airbnbs-server-driven-ui-system-842244c5f5),
[Yelp CHAOS](https://engineeringblog.yelp.com/2024/03/chaos-yelps-unified-framework-for-server-driven-ui.html),
[Duolingo](https://blog.duolingo.com/server-driven-ui/),
[Shopify](https://shopify.engineering/server-driven-ui-in-shop-app),
[DoorDash](https://careersatdoordash.com/blog/improving-development-velocity-with-generic-server-driven-ui-components/),
[Spotify HubFramework](https://github.com/spotify/HubFramework),
[Uber Screenflow retro](https://artem-tyurin.medium.com/screenflow-an-unfinished-attempt-at-a-cross-platform-server-driven-ui-at-uber-749c1bc1d89).

## 3. Core model

Three tiers — the convergent shape across every successful implementation:

```
Screen  →  Sections (flat ordered list)  →  Section config (visibility, mode, props)
```

Flat section lists, not deep layout trees. Nesting is allowed only *inside* a
component where genuinely needed.

### 3.1 Screen config schema (illustrative)

```json
{
  "screen": "reality/listing-detail",
  "version": 42,
  "sections": [
    { "type": "gallery.v1",         "visible": true },
    { "type": "price-box.v1",       "visible": true },
    { "type": "agent-contact.v1",   "visible": true,
      "mode": "sticky-sidebar",
      "overrides": { "mobile": { "mode": "bottom-bar" } } },
    { "type": "similar-listings.v1","visible": false },
    { "type": "mortgage-calc.v1",   "visible": true,
      "props": { "maxYears": 30 } }
  ]
}
```

Decisions baked into this shape:

- **Semantic, versioned type names** (`price-box.v1`). Breaking changes mint
  `price-box.v2`; published contracts are never mutated. Otherwise evolution
  is additive-only (clients ignore unknown fields).
- **One base config + sparse per-platform overrides**, not forked configs.
  The server resolves `base + platform (+ later: audience/flag)` at request
  time. Screen IDs should reuse the existing `docs/screens/<product>/<id>`
  catalog.
- **Display modes are constrained enums declared by the component**, not free
  CSS. The frontend registry is the source of truth for which modes each
  component supports; the admin can only pick from those.
- **`required` is a property of the component (registry), not the config** —
  an operator cannot make a component optional by editing config.

### 3.2 Component registry

Each frontend ships a registry:

```
type string → { renderer, supportedModes, required, propSchema, fallback }
```

Each platform also **publishes its registry manifest** (types + modes + prop
schemas + minimum app version) to the backend — ideally generated through the
existing TypeSpec → client pipeline — so the superadmin editor knows exactly
what every platform supports without hardcoding.

## 4. Resilience rules ("hiding never breaks the page")

1. **Unknown type never crashes.** Registry miss → render nothing (optional)
   or placeholder (required); log to analytics with config version + client
   version. Preferred at scale: **server-side filtering** — the client
   advertises supported components/versions in a request header and the
   server omits what it can't render (Yelp's Register model). Critical for
   RN/KMP binaries that may be months stale.
2. **Hidden required component → placeholder.** A neutral "section
   unavailable" block reserving sensible space. In the editor, required
   sections carry a lock badge and simply have **no hide/delete affordance**
   (Gutenberg lesson: no disabled buttons, no hidden unlock flows).
3. **Hidden optional component → absent from the tree** (not
   `visibility:hidden`), and **containers own inter-section spacing via CSS
   `gap`**; section components ship zero external margin. This single rule
   eliminates orphaned margins/double spacing when anything is removed.
4. **Per-section error boundaries** (React) / isolated composables (Compose):
   a crashing section degrades to its fallback, never the page. Optional
   section fails → collapse; required section fails → placeholder.
5. **Stale-client strategy (Duolingo):** version UI config and data
   independently. A client below the config's minimum schema version keeps
   rendering its **last-known-good cached layout** with fresh data.
6. **Mobile activation timing:** never swap layout mid-session. Fetch config
   in the background, **activate on next launch / next screen entry**
   (Firebase Remote Config's hard-won rule). Ship a compiled-in default
   config as the final offline/first-run fallback.

## 5. Superadmin editor (admin-web)

Model: **inline live preview + tree panel + constrained slots** —
Storyblok/Puck-style, not free-form Webflow-style. [Puck](https://puckeditor.com)
(MIT, React) is the best open reference and a candidate to embed rather than
build from scratch.

### 5.1 Preview bridge

- Iframe the **real site in draft mode**; postMessage bridge with origin
  validation + handshake.
- We own both sides → tag rendered sections with `data-section-id` (skip
  Sanity-style stega string encoding).
- Editor pushes the **full draft config** into the iframe on every change;
  the site re-renders optimistically (Storyblok's protocol — simplest robust
  option). Persisted only on save/publish.
- Draft-mode cookie/flag so the framed site fetches draft config.

### 5.2 Editing surface

- **Canvas overlays + labeled tree panel, synced both ways**: click a section
  on canvas → tree row highlights + config panel opens; hover a tree row →
  outline on canvas.
- Tree rows carry the direct controls: **eye toggle** (visibility), **drag
  handle + up/down arrows** (reorder; arrows double as the accessible
  non-drag path), **mode dropdown** (populated from the registry manifest),
  **lock badge** on required sections.
- **Platform switcher** in the toolbar (web / mobile), like a breakpoint
  switcher. Sections hidden on the current platform render **dimmed on
  canvas**, not removed. Overridden sections get a badge + "reset to base".

### 5.3 Workflow & governance

- State machine: **Draft → (preview link) → Published**; optional scheduled
  publish later.
- Every publish creates an **immutable version**; one-click rollback; audit
  log (who/what/when + diff).
- **Publish is gated by server-side validation** — hard guarantee, not a
  lint: schema-valid; every referenced type exists in every target platform's
  registry; required sections present and visible; modes ∈ supported set;
  props validate against the component's prop schema. Errors block publish;
  warnings don't (Sanity model).
- Roles: **schema/registry authors** (developers, via code) vs **content
  operators** (superadmin, arrange within the rails).

## 6. Placement in the PPT architecture

| Piece | Where | Notes |
|---|---|---|
| Control plane (CRUD, draft/publish, versions, audit, validation) | **api-server** | `layout_configs` + `layout_config_versions` tables; superadmin-scoped routes behind existing admin auth |
| Resolved read endpoint | **api-server** (ppt/acc screens) + **reality-server** (reality screens) | `GET /layout/{screen}?platform=…&app_version=…` → resolved section list; aggressively cacheable, version-bumped on publish |
| Editor UI | **admin-web** | iframe bridge + tree panel (§5) |
| reality-web delivery | Next.js ISR | publish webhook → `revalidateTag('layout:{screen}')`; keep a long time-based revalidate as safety net |
| ppt-web / accounting-web delivery | TanStack Query | fetch on navigation, cache last-known-good |
| Mobile delivery (RN + KMP) | local cache | background fetch, activate next launch/screen; compiled-in default config |
| Registry manifests | TypeSpec pipeline | generated alongside `@ppt/api-client`; mobile-native via openapi-generator |
| Screen catalog | `docs/screens/` | share screen IDs with the existing screen-map system |
| Feature flags | separate layer | flags gate *whether* (kill switch, rollout %); layout config defines *what/how*. A section may reference a `flagId`; the resolver applies it. Do not merge the two systems |

## 7. Known costs & pitfalls

- **Complexity moves server-side; it doesn't disappear.** Resolvers absorb
  override merging, version filtering, flag substitution, fallback logic.
- **Testing surface multiplies** (screens × platforms × client versions ×
  configs). Mitigation: log every section render with config + client
  version; dashboard failure rates per (component, client-version) cell.
- **Editor last, contract first.** DoorDash's ordering: harden the config
  contract on one surface for a long time *before* building the GUI composer.
- **Out of scope surfaces:** deep-native views (map), auth flows, low-churn
  screens (settings) — no velocity to gain, real downside risk.
- **Skeletons vs collapse:** skeletons are for *loading* with predictable
  dimensions; a *hidden* section must collapse entirely — a skeleton implies
  content is coming.

## 8. Rollout plan

1. **Contract first** — section-list schema + registry pattern on **one**
   high-churn pilot screen: `reality/listing-detail` (exists on web + KMP).
2. Resolved-layout endpoint + defensive rendering (gap spacing, error
   boundaries, unknown-type fallback, required-placeholder) in reality-web.
3. **Minimal superadmin** — tree panel, eye toggles, reorder,
   publish/rollback. No iframe preview yet.
4. Live iframe preview bridge; platform switcher + overrides; display modes.
5. Extend to reality landing + list pages; then ppt-web and accounting-web
   screens; then mobile registry manifests + KMP renderer wiring.

## 9. Open questions

- Embed Puck for the editor vs build the tree-panel editor natively in
  admin-web? (Puck saves DnD/slot plumbing but adds a dependency and its
  config model would need mapping onto ours.)
- Should the resolved-layout endpoint live in each server, or in a single
  shared crate consumed by api-server + reality-server (+ accounting-server
  later)?
- Audience targeting (per-org / per-segment layouts) — in scope for v2, and
  does it interact with RLS/tenant context?
- How registry manifests for KMP are published — build-time upload step vs
  checked-in generated file.

## 10. Further reading

- [Airbnb Ghost Platform deep dive](https://medium.com/airbnb-engineering/a-deep-dive-into-airbnbs-server-driven-ui-system-842244c5f5)
- [Yelp CHAOS (2024)](https://engineeringblog.yelp.com/2024/03/chaos-yelps-unified-framework-for-server-driven-ui.html) · [backend (2025)](https://engineeringblog.yelp.com/2025/07/chaos-inside-yelps-sdui-framework.html)
- [Duolingo SDUI](https://blog.duolingo.com/server-driven-ui/)
- [Shopify Shop app SDUI](https://shopify.engineering/server-driven-ui-in-shop-app)
- [Lyft Canvas (protobuf SDUI)](https://eng.lyft.com/the-journey-to-server-driven-ui-at-lyft-bikes-and-scooters-c19264a0378e)
- [Puck editor — Slots API](https://puckeditor.com/docs/api-reference/fields/slot)
- [Contentful editable patterns (lock/whitelist UX)](https://www.contentful.com/help/studio/experiences/editable-patterns/)
- [Sanity visual-editing architecture](https://www.sanity.io/docs/visual-editing/visual-editing-architecture)
- [Storyblok preview bridge](https://www.storyblok.com/docs/libraries/js/preview-bridge)
- [Firebase Remote Config loading strategies](https://firebase.google.com/docs/remote-config/loading)
- [DivKit (Yandex, OSS native SDUI)](https://github.com/divkit/divkit)
