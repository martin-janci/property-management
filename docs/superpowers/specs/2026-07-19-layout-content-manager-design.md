# Layout & Content Manager — System Design

**Date:** 2026-07-19 (revised same day after brainstorming session)
**Status:** Design approved in brainstorm; awaiting implementation plan
**Scope:** All four apps (ppt-web, reality-web, accounting-web, mobile RN) + KMP mobile-native; superadmin editor in admin-web; tenant-scoped editor in ppt-web

## 1. Problem statement

We want a universal framework to control **which components appear on which
pages, in what order, and in what display mode** — across the Reality portal,
the Property Management app, the Accounting app, and their mobile
counterparts.

V1 must do three jobs (decided in brainstorm):

1. **Operator tuning** — superadmin adjusts what each site shows (hide a
   section, reorder, switch a list page to grid) without a deploy.
2. **Per-tenant customization** — organizations get customized layouts,
   edited both by the superadmin *and* self-service by tenant org admins
   within superadmin-defined rails.
3. **Kill-switch** — a broken or problematic section can be pulled from
   production in one action, propagating on next load.

Out of scope for v1: A/B testing / experimentation (a section may later
reference a feature flag, but no flag system ships in v1).

Cross-cutting requirements:

- **Never break a page.** Hiding an optional component collapses cleanly;
  hiding/killing a mandatory component renders a placeholder instead.
- Per-platform customization (web vs mobile) without forking page configs.
- Draft → publish workflow, immutable versions, rollback, audit.

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
- **One base config + sparse overrides**, not forked configs. Screen IDs
  reuse the existing `docs/screens/<product>/<id>` catalog.
- **Display modes are constrained enums declared by the component**, not free
  CSS. The frontend registry is the source of truth for which modes each
  component supports; editors can only pick from those.
- **`required` is a property of the component (registry), not the config** —
  no editor can make a required component optional.

### 3.2 Resolution model

Resolution is a layered merge, computed server-side per request in a shared
`layout-core` backend crate (consumed by api-server and reality-server,
mirroring the existing `api-core` pattern):

```
platform default → superadmin base config → platform override
                → tenant (org) override   → kill flags
```

- Tenant override rows carry `org_id` and hold only the delta (visibility,
  order, mode, whitelisted props) — never a full forked config.
- The resolved endpoint is cacheable per `(screen, platform, org_id,
  config_version)`.
- Reality-portal public screens have no org dimension unless an agency
  context applies; the tenant layer simply doesn't contribute there.

### 3.3 Component registry

Each frontend ships a registry:

```
type string → { renderer, supportedModes, required, propSchema, fallback }
```

Each platform also **publishes its registry manifest** (types + modes + prop
schemas + minimum app version) to the backend — generated through the
existing TypeSpec pipeline for TS apps; for KMP, a checked-in generated file
updated by the SDK-generation step. The editors read these manifests, so
nothing about platform capabilities is hardcoded in admin UI.

### 3.4 Editing rails (two-role model)

- **Superadmin (rail author)** — defines, per screen: the section set,
  defaults, per-platform overrides, which optional sections tenants may
  toggle, and a **per-prop editability whitelist** (which props tenant
  admins may set, with allowed ranges where relevant).
- **Tenant org admin (operator)** — customizes their org's screens within
  those rails: show/hide allowed optional sections, reorder, pick display
  modes, and edit whitelisted props. Cannot add unknown sections, touch
  required sections, or edit non-whitelisted props — enforced server-side at
  save time, not just hidden in the UI.

This follows Contentful's editable-patterns model: the pattern author
explicitly whitelists what instance editors may change.

## 4. Resilience rules ("hiding never breaks the page")

1. **Unknown type never crashes.** Registry miss → render nothing (optional)
   or placeholder (required); log to analytics with config version + client
   version. Preferred at scale: **server-side filtering** — the client
   advertises supported components/versions in a request header and the
   server omits what it can't render (Yelp's Register model). Critical for
   RN/KMP binaries that may be months stale.
2. **Hidden or killed required component → placeholder.** A neutral "section
   unavailable" block reserving sensible space. In the editors, required
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

## 5. Kill-switch

Built into the layout system, not delegated to a feature-flag service
(decided in brainstorm — no external flag infra in v1):

- Every section instance has a `killed` flag settable by superadmin in one
  action, **bypassing the draft → publish gate** (it is an operational
  control, not an editorial change). Killing a required section makes it
  render its placeholder everywhere.
- Kill state is stored alongside (not inside) published config versions, so
  publishing or rolling back a config does not accidentally resurrect a
  killed section; un-kill is an equally explicit action, and both are
  audit-logged.
- Propagation: next navigation / ISR revalidation on web (≤ ~1 min), next
  launch or screen entry on mobile. No real-time push channel and no
  mid-session layout swaps in v1.

## 6. Editors

One shared **`@ppt/layout-editor`** React package (in `frontend/packages/`),
mounted in two places with different capability sets:

- **admin-web — full superadmin editor:** rail authoring (section set,
  required flags via registry, tenant-editable whitelists, per-prop
  whitelists), base config editing, per-platform overrides, per-tenant
  override editing, kill-switch, publish/rollback/audit.
- **ppt-web — scoped tenant editor:** the same tree-panel component
  restricted to the tenant's rails (visibility, order, modes, whitelisted
  props on their org's screens). Server-side enforcement of the rails at
  save time.

Model: **tree panel + live iframe preview** (Storyblok/Puck-style, decided
over embedding Puck — our section-list model is simpler than Puck's
free-form target, and the two-role rails model needs custom permission
logic either way). **No canvas drag-and-drop in v1** — tree reorder with
drag handle + up/down arrows covers it (arrows double as the accessible
path).

### 6.1 Preview bridge

- Iframe the **real site in draft mode**; postMessage bridge with origin
  validation + handshake.
- We own both sides → tag rendered sections with `data-section-id` (skip
  Sanity-style stega string encoding).
- Editor pushes the **full draft config** into the iframe on every change;
  the site re-renders optimistically (Storyblok's protocol — simplest robust
  option). Persisted only on save/publish.
- Draft-mode cookie/flag so the framed site fetches draft config.
- Canvas overlays + tree panel synced both ways: click a section on canvas →
  tree row highlights + config panel opens; hover a tree row → outline on
  canvas.

### 6.2 Editing surface

- Tree rows carry the direct controls: **eye toggle** (visibility), **drag
  handle + arrows** (reorder), **mode dropdown** (populated from the
  registry manifest), **prop form** (from the component's prop schema,
  filtered by whitelist in the tenant editor), **lock badge** on required
  sections, **kill badge** on killed sections (superadmin only).
- **Platform switcher** in the toolbar (web / mobile). Sections hidden on
  the current platform render **dimmed on canvas**, not removed. Overridden
  sections get a badge + "reset to base"; in superadmin's per-tenant view,
  tenant-overridden sections badge the same way.

### 6.3 Workflow & governance

- State machine: **Draft → (preview link) → Published**; optional scheduled
  publish later. Kill-switch bypasses this (§5).
- Every publish creates an **immutable version**; one-click rollback; audit
  log (who/what/when + diff) covering superadmin edits, tenant edits, and
  kill/un-kill actions.
- **Publish is gated by server-side validation** — hard guarantee, not a
  lint: schema-valid; every referenced type exists in every target platform's
  registry; required sections present and visible; modes ∈ supported set;
  props validate against the component's prop schema; tenant saves
  additionally validate against the rails. Errors block publish; warnings
  don't (Sanity model).

## 7. Placement in the PPT architecture

| Piece | Where | Notes |
|---|---|---|
| Resolution logic (merge, filtering, validation) | **`layout-core` crate** (backend workspace) | shared by api-server + reality-server (+ accounting-server later), mirroring `api-core` |
| Control plane (CRUD, rails, tenant overrides, kill, versions, audit) | **api-server** | `layout_configs`, `layout_config_versions`, `layout_tenant_overrides` (org_id-scoped), `layout_kill_flags`; superadmin routes behind admin auth, tenant routes behind org admin auth |
| Resolved read endpoint | **api-server** (ppt/acc screens) + **reality-server** (reality screens) | `GET /layout/{screen}?platform=…&app_version=…` (+ org from tenant context) → resolved section list; cacheable per (screen, platform, org, version) |
| Superadmin editor | **admin-web** | full `@ppt/layout-editor` |
| Tenant editor | **ppt-web** | scoped `@ppt/layout-editor` |
| reality-web delivery | Next.js ISR | publish/kill webhook → `revalidateTag('layout:{screen}')`; long time-based revalidate as safety net |
| ppt-web / accounting-web delivery | TanStack Query | fetch on navigation, cache last-known-good |
| Mobile delivery (RN + KMP) | local cache | background fetch, activate next launch/screen; compiled-in default config |
| Registry manifests | TypeSpec pipeline (TS apps); checked-in generated file (KMP) | editors read manifests, nothing hardcoded |
| Screen catalog | `docs/screens/` | share screen IDs with the existing screen-map system |
| Feature flags / A/B | **not in v1** | schema reserves an optional `flagId` per section for later; no flag service shipped |

## 8. Known costs & pitfalls

- **Complexity moves server-side; it doesn't disappear.** The resolver
  absorbs override merging, rails enforcement, version filtering, kill
  application, fallback logic. `layout-core` needs thorough unit tests over
  the merge precedence.
- **Testing surface multiplies** (screens × platforms × client versions ×
  orgs). Mitigation: log every section render with config + client version;
  dashboard failure rates per (component, client-version) cell.
- **Editor last, contract first.** DoorDash's ordering: harden the config
  contract on the pilot screens before investing in editor polish.
- **Out of scope surfaces:** deep-native views (map), auth flows, low-churn
  screens (settings) — no velocity to gain, real downside risk.
- **Skeletons vs collapse:** skeletons are for *loading* with predictable
  dimensions; a *hidden* section must collapse entirely — a skeleton implies
  content is coming.
- **Two editors, one package:** the tenant editor must be a capability-
  restricted mount of the same component, not a fork — divergence between
  the two editing surfaces is the maintenance trap here.

## 9. Rollout plan

Pilot = **two screens, one per architectural risk** (decided in brainstorm):
`ppt/dashboard` (tenant overrides + both editors) and
`reality/listing-detail` (public SSR/ISR + KMP delivery).

1. **Contract first** — section-list schema, `layout-core` merge resolver
   with full precedence tests, registry pattern + manifests for ppt-web and
   reality-web.
2. **Defensive rendering** on both pilot screens (gap spacing, error
   boundaries, unknown-type fallback, required-placeholder), resolved-layout
   endpoints, ISR revalidation hook.
3. **Superadmin editor MVP** in admin-web — tree panel, visibility, reorder,
   modes, props, publish/rollback, kill-switch. No iframe preview yet.
4. **Tenant editor** in ppt-web — scoped mount + server-side rails
   enforcement + rails authoring UI in admin-web.
5. **Live iframe preview bridge** + platform switcher/overrides.
6. Expand: reality landing + list pages → remaining ppt-web screens →
   accounting-web → mobile registry manifests + RN/KMP renderer wiring.

## 10. Resolved & open questions

Resolved in the 2026-07-19 brainstorm:

- **Drivers:** operator tuning + per-tenant customization + kill-switch;
  A/B out of v1.
- **Tenant editing:** self-service from day one, two-role rails model.
- **Tenant powers:** visibility + order + modes + per-prop-whitelisted props.
- **Pilot:** `ppt/dashboard` + `reality/listing-detail`.
- **Editor:** custom shared `@ppt/layout-editor`; Puck not embedded; no
  canvas DnD in v1.
- **Kill-switch:** built-in, next-load propagation, publish-gate bypass.
- **Resolver placement:** shared `layout-core` crate.
- **KMP manifests:** checked-in generated file.

Still open (fine to settle during implementation planning):

- Whether accounting-web joins the pilot expansion before or after mobile.
- Whether tenant overrides live under RLS like other org data or in
  admin-owned tables with explicit org_id filtering (leaning RLS for
  consistency with the rest of the schema).
- Draft-preview auth for the iframe (signed preview token vs admin session
  cookie pass-through).

## 11. Further reading

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
