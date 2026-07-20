# Layout Mobile Registries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the two mobile clients into the layout system (spec §9 step 6): the RN management app renders its dashboard through a section registry fed by `GET /api/v1/layout/resolved/ppt/dashboard?platform=mobile` (last-known-good cached, activate-on-next-launch), and the KMP Reality app renders listing-detail section order/visibility from reality-server's public resolved endpoint (compiled-in default, fetch-at-entry). A canonical checked-in mobile manifest covers both apps' section types.

**Architecture:** (1) RN: a `features/layout` module using the app's own `apiRequest` layer (NOT `@ppt/api-client` — its relative URLs/token provider don't work on RN), an AsyncStorage-backed cache with strict activate-on-launch semantics (render what was cached at mount; fetch in background only to serve the NEXT launch — never swap mid-session, spec §4.6), an RN `LayoutSections` renderer (order, placeholder, unknown-skip-warn-once) over two managed sections (`dashboard-stats.v1`, `action-queue.v1` — the announcements/quick-actions blocks stay hardcoded below the managed area for now). (2) KMP shared: a `layout` package (serialization models, `LayoutRepository` reusing the `HttpClientProvider` singleton, compiled-in `DEFAULT_LISTING_DETAIL_LAYOUT`, a `resolveEffectiveLayout` helper with fetch-once-at-entry semantics), fully covered by commonTest (MockEngine). (3) KMP Android: `ListingDetailContent` iterates the resolved section list through a `when(type)` registry dispatch (gallery/agent-contact/listing-header/key-details/description mapped to existing composables; features/additional-info/resources are declared-but-folded — they render inside the description tabs regardless, documented), placeholder composable for `presentation=placeholder`, unknown types skipped. (4) **iOS is explicitly OUT** — no macOS builder available here; the SwiftUI dispatch is a follow-up with the same shared models. (5) A canonical mobile manifest JSON checked into the RN app covering BOTH apps' types, consistency-tested.

**Tech Stack:** RN/Expo + jest-expo + AsyncStorage; Kotlin Multiplatform (kotlinx-serialization, Ktor + MockEngine, coroutines-test); Compose (Android).

## Global Constraints

- Spec: `…/2026-07-19-layout-content-manager-design.md` §4.5–4.6 (stale-client + activation timing), §7 (mobile delivery row), §9 step 6. All seven prior slices merged on `dev` (#2424–#2431).
- **Branch:** `feature/layout-mobile-registries` from `dev`.
- **Section-type contract:** mobile implements the EXISTING type names. RN dashboard: `dashboard-stats.v1` (wraps today's stats grid), `action-queue.v1` (wraps today's Pending Actions block). KMP listing-detail: `gallery.v1`→HeroGallery, `listing-header.v1`→HeaderSection, `key-details.v1`→QuickStatsStrip, `description.v1`→TabStrip+tab content, `agent-contact.v1`→StickyAgentBar+BottomActionBar; `features.v1`/`additional-info.v1`/`resources.v1` are REGISTERED no-op types on mobile (their content lives inside the description tabs) — required because `validate_publish` checks every config section against EVERY stored manifest, so the mobile manifest must know all eight or publishing the reality base config breaks.
- **Activation semantics (both apps):** never swap layout mid-session. RN: read cached layout synchronously-ish at mount (AsyncStorage read before first managed render — render default until the read resolves, which is one frame, then STAY on whatever that read produced; the network fetch stores only for next launch). KMP: fetch once at screen entry with the repository's normal error handling; on any failure use the compiled default; no re-fetch while the screen is open.
- **Defensive rules identical to web:** unknown type → skip + warn once; `presentation=placeholder` → neutral placeholder (i18n'd RN / stringResource-or-hardcoded-sk Android matching app conventions); malformed payload → default layout; never crash, never blank.
- **RN fetch:** via `useApi.ts`'s `apiRequest` (absolute URL, SecureStore token, X-Tenant-ID from JWT) — do NOT import the api-client layout domain. Cache key in `src/services/localCacheKeys.ts` per its conventions (`ppt_layout_<screen>` style — match the file's naming).
- **KMP:** follow house patterns exactly — `@Serializable` + explicit `@SerialName` for snake_case wire fields, constructor-injected repository using `HttpClientProvider.client`, `ignoreUnknownKeys` already global; `presentation` enum with `@SerialName("visible")`/`@SerialName("placeholder")` and an UNKNOWN default via `coerceInputValues`-safe design (ADAPT: check how existing enums handle unknown wire values with `ignoreUnknownKeys`; if enums throw on unknown values, model `presentation` as String + helper — report the choice). Tests in commonTest with MockEngine mirroring `ListingDetailRepositoryTest.kt`.
- **Canonical mobile manifest:** `frontend/apps/mobile/src/features/layout/mobile-manifest.json` — platform `mobile`, components: the two dashboard types (required: true both) + all eight listing-detail types (gallery + listing-header required, rest optional; the three folded types optional). RN test asserts (a) dashboard types === RN registry keys, (b) the eight listing-detail types match a hardcoded list with a comment pointing at the KMP registry file (documented duplication — KMP tests can't read this JSON).
- Gates: `cd frontend && pnpm -F @ppt/mobile test` + `pnpm check && pnpm typecheck`; `cd mobile-native && ./gradlew :shared:allTests` and `./gradlew build` (CI runs these in mobile-native.yml). Known pre-existing failures untouched. NO backgrounded gate commands; no piping cargo/gradle through tail.
- Commit scopes: `feat(mobile)`, `feat(mobile-native)`, `docs(...)`. ADAPT rule as before; report adaptations.

## File Structure

```
frontend/apps/mobile/src/features/layout/types.ts          # ResolvedScreen/Section types (local)
frontend/apps/mobile/src/features/layout/layoutCache.ts    # AsyncStorage last-known-good + activate-on-launch
frontend/apps/mobile/src/features/layout/registry.tsx      # dashboardRegistry (2 types) + DEFAULT_DASHBOARD_LAYOUT
frontend/apps/mobile/src/features/layout/LayoutSections.tsx# RN renderer (order/placeholder/unknown-skip)
frontend/apps/mobile/src/features/layout/useDashboardLayout.ts # mount-read + background-refresh hook
frontend/apps/mobile/src/features/layout/mobile-manifest.json
frontend/apps/mobile/src/features/layout/*.test.tsx        # renderer/cache/hook/manifest tests
frontend/apps/mobile/src/screens/dashboard/DashboardScreen.tsx  # refactor: managed area via LayoutSections
frontend/apps/mobile/src/services/localCacheKeys.ts        # + layout cache key
mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/layout/LayoutModels.kt
mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/layout/LayoutRepository.kt
mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/layout/DefaultLayout.kt
mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/layout/LayoutRepositoryTest.kt
mobile-native/shared/src/commonTest/kotlin/three/two/bit/ppt/reality/layout/LayoutModelsContractTest.kt
mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/listing/ListingSectionRegistry.kt
mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/listing/ListingDetailContent.kt  # dispatch refactor
mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/ui/listing/ListingDetailScreen.kt   # layout fetch wiring
docs/repo-map.md
```

---

### Task 1: RN layout module + dashboard refactor (TDD)

**Files:** all `frontend/apps/mobile` files above.

**Interfaces / behavior contract:**
- `types.ts`: `ResolvedSection { type: string; mode?: string; props?: Record<string, unknown>; presentation: 'visible' | 'placeholder' }`, `ResolvedScreen { screen: string; version: number; sections: ResolvedSection[] }` (local copies — mobile doesn't depend on the api-client layout domain).
- `layoutCache.ts`: `readCachedLayout(screen): Promise<ResolvedScreen | null>` (AsyncStorage `JSON.parse` guarded — malformed → null + remove), `writeCachedLayout(screen, layout)`, key from `localCacheKeys.ts`. Validation: shape-check `screen` match + `Array.isArray(sections)` before returning.
- `useDashboardLayout(screen)`: state starts as `DEFAULT_DASHBOARD_LAYOUT`; on mount, `readCachedLayout` → if non-null, set state ONCE (the activation moment — this is launch-time activation, allowed); then fire `apiRequest<ResolvedScreen>('/api/v1/layout/resolved/'+screen+'?platform=mobile')` in the background — on success shape-check and `writeCachedLayout` ONLY (do NOT set state); failures silent (`console.warn` once). Result: session renders cache-or-default; fresh layout appears next launch (spec §4.6). Return `{ layout }`.
- `registry.tsx`: `dashboardRegistry` mapping `dashboard-stats.v1` → the extracted stats-grid component and `action-queue.v1` → the extracted Pending Actions component (extract both from `DashboardScreen` verbatim, preserving their queries/props — components receive `{ mode?, props? }` and internally keep their existing hooks/callbacks; ADAPT to how the screen currently passes `onNavigate` — thread it via a React context or renderer prop, smallest clean change, report it). `DEFAULT_DASHBOARD_LAYOUT` lists both visible.
- `LayoutSections.tsx`: mirrors the web renderers' semantics — order-preserving, `presentation=placeholder` → `<View accessibilityRole="alert"-free placeholder>` with i18n'd title/body (add `layout.placeholderTitle`/`layout.placeholderBody` keys to ALL SIX locale files under `src/locales/` — same keys as web apps), unknown → skip + warn once, container spacing via the parent's existing section styles (`gap`-based wrapper style).
- `DashboardScreen.tsx`: header stays; the stats + pending-actions area becomes `<LayoutSections layout={layout} registry={dashboardRegistry} …/>`; announcements + quick-actions remain hardcoded below (deliberate — not yet managed). Existing DashboardScreen tests must stay green (ADAPT their mocks minimally; report).
- `mobile-manifest.json` + `manifest.test.ts` per Global Constraints.
- Tests (jest-expo, mirror `DashboardScreen.test.tsx` mocking conventions): renderer order/placeholder/unknown-skip (3+), cache read/write/malformed (3), hook activation semantics — cached layout activates at mount, successful background fetch does NOT change state but writes cache, fetch failure silent (3, use fake timers/`waitFor`), manifest consistency (1), DashboardScreen renders managed sections via default layout with fetch mocked out (existing + 1).

- [ ] TDD; verify FOREGROUND and WAIT: `cd frontend && pnpm -F @ppt/mobile test` (full app suite; report pre-existing failures if any — none are known for mobile) + `pnpm typecheck` + Biome on touched files.
- [ ] Commit — `feat(mobile): dashboard renders through cached resolved layout with next-launch activation`

---

### Task 2: KMP shared layout package (TDD)

**Files:** the three `commonMain` files + two `commonTest` files above.

**Interfaces / behavior contract:**
- `LayoutModels.kt`: `@Serializable data class ResolvedLayoutSection(val type: String, val mode: String? = null, @SerialName("props") val props: JsonObject? = null, val presentation: String = "visible")` and `@Serializable data class ResolvedLayoutScreen(val screen: String, val version: Int, val sections: List<ResolvedLayoutSection>)` — presentation as String per the enum-unknown-value concern (helpers `isPlaceholder`/`isVisible`; unknown presentation values treated as visible — defensive). ADAPT if house style prefers enums with a safe fallback and it's provably safe under the shared Json config; report.
- `DefaultLayout.kt`: `val DEFAULT_LISTING_DETAIL_LAYOUT = ResolvedLayoutScreen("reality/listing-detail", 0, listOf(…))` — the eight types in web base order, all visible.
- `LayoutRepository.kt` (constructor-injected `HttpClient = HttpClientProvider.client`, `baseUrl = ApiConfig.baseUrl` — mirror `ListingRepository`'s construction exactly): `suspend fun getResolvedLayout(screen: String): ResolvedLayoutScreen` — GET `$baseUrl/api/v1/layout/resolved/$screen?platform=mobile` (public, no auth header), on ANY failure (non-2xx, network, decode, wrong screen echo, empty sections list is ALLOWED) return `DEFAULT_LISTING_DETAIL_LAYOUT` when screen == its screen else a minimal default — simplest: `getListingDetailLayout(): ResolvedLayoutScreen` fixed to the one screen, never throws. Path-encode the screen segment like other repos (`asPathSegment` on each `/`-part — check `UrlEncoding.kt`; the screen contains a literal `/` which must stay a path separator: build the URL from the two segments, ADAPT to the resolved endpoint's catch-all expectations — `reality%2Flisting-detail` would NOT match the axum `{*screen}` route the same way; use the literal path `layout/resolved/reality/listing-detail`).
- Tests mirroring `ListingDetailRepositoryTest.kt` (MockEngine): 200 decode (order + presentation + unknown-extra-fields tolerated), non-2xx → default, malformed JSON → default, decode of `presentation:"placeholder"` + unknown presentation string → visible; `LayoutModelsContractTest.kt` pins the wire shape (encode/decode round-trip of a fixture matching the server's serde output).

- [ ] TDD; verify FOREGROUND and WAIT: `cd mobile-native && ./gradlew :shared:allTests` (plus `./gradlew :shared:build` if fast). Report test counts.
- [ ] Commit — `feat(mobile-native): shared layout models, repository and compiled default`

---

### Task 3: KMP Android dispatch (registry refactor)

**Files:** `ListingSectionRegistry.kt` (new), `ListingDetailContent.kt` + `ListingDetailScreen.kt` (refactor).

**Interfaces / behavior contract:**
- `ListingSectionRegistry.kt`: `object ListingSectionRegistry { val supportedTypes: Set<String> = setOf(all eight) }` + a `@Composable fun LazyListScope-extension`-style dispatcher OR a plain `fun sectionItems(scope: LazyListScope, section: ResolvedLayoutSection, ctx: ListingSectionContext)` where `ListingSectionContext` bundles the params today's composables need (listing, callbacks, tab state…) — ADAPT to the existing `ListingDetailContent` parameter surface with the smallest clean restructure; the `when(section.type)` maps the five rendering types to the existing composable calls IN THEIR CURRENT FORM (no visual changes under the default layout), the three folded types to no-ops, unknown → no-op (+ one `println`/log warn — match app logging conventions), `presentation == placeholder` → `PlaceholderSection()` composable (neutral card, ~96dp min height, text "Sekcia nie je dostupná" — ADAPT to the app's string-resource conventions; if all strings are resources, add one).
- `ListingDetailContent.kt`: the fixed `item { … }` list becomes iteration over `layout.sections` dispatching through the registry. Sticky/bottom-bar special case: `agent-contact.v1` controls BOTH `StickyAgentBar` and `BottomActionBar` visibility (hidden/killed → neither renders; placeholder → placeholder card in the list, no bars).
- `ListingDetailScreen.kt`: obtain the layout via `remember { LayoutRepository(...) }` + `LaunchedEffect(listingId) { layout = repo.getListingDetailLayout() }` starting from `DEFAULT_LISTING_DETAIL_LAYOUT` — fetch-once-at-entry; the state may settle once shortly after entry (network vs default — acceptable at screen entry per spec §4.6's "next screen entry" activation; no re-fetch afterwards). ADAPT to how `ListingDetailViewModel` is wired if hosting the fetch there is cleaner — report the choice.
- Behavior under the default layout must be pixel-identical to today (this is a pure restructure + dispatch).
- Tests: shared logic is covered by Task 2; Android-side has no unit-test harness for composables (no androidTest set) — the gate is `./gradlew build` (compiles both apps) + `:shared:allTests`. State this explicitly in the report.

- [ ] Verify FOREGROUND and WAIT (generous timeout — gradle builds are slow): `cd mobile-native && ./gradlew build` green.
- [ ] Commit — `feat(mobile-native): listing detail renders through the shared resolved layout`

---

### Task 4: Gates + docs

- `docs/repo-map.md` layout bullet: `Mobile: RN features/layout (dashboard, cached next-launch activation) + mobile-native shared/layout + Android registry dispatch; canonical mobile manifest in apps/mobile.`
- Screen-map Agent Log entries where docs exist (ppt dashboard mobile note; reality listing-detail mobile note) — skip+note absences.
- Full gates: `cd frontend && pnpm check && pnpm typecheck && pnpm -F @ppt/mobile test`; `cd mobile-native && ./gradlew build :shared:allTests`. Known pre-existing failures only.
- Commit — `docs(repo-map): layout mobile registries pointers`

---

## Deliberate scope decisions (do not "fix" during implementation)

- **iOS renderer is OUT** — no macOS builder in this environment; the shared models/repository are iOS-ready, and the SwiftUI `switch` dispatch is a documented follow-up.
- **KMP has no persistent last-known-good cache** — no settings/DB lib exists in shared; fetch-at-entry + compiled default is the v1 semantics; persistence (multiplatform-settings) is a follow-up.
- **RN manages only the stats + action-queue area**; announcements/quick-actions blocks stay hardcoded until those section types exist across the system.
- **The three folded KMP types render no-op** — their content ships inside the description tabs; granular mobile control of them is future work.
- **No mode support on mobile v1** — `mode` is carried through but no mobile section declares modes in the manifest.

## Out of scope (subsequent plans)

iOS SwiftUI dispatch; KMP layout persistence; `layout_editor_*` capability; per-tenant preview; ppt-web frame-protection follow-up.
