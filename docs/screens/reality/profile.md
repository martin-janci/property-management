---
id: reality/profile
name: Profile
product: reality
implementations:
  reality-web:
    component: ProfilePage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: stub
  mobile-native:
    component: AccountScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: partial
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/favorites
    rel: child
  - id: reality/saved-searches
    rel: child
  - id: reality/inquiries
    rel: child
sharedComponents:
  - portal-header
  - portal-footer
  - tabs
  - listing-card
  - timeline
  - kv-list
  - settings-side-nav
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/profile.html
    frame: profile-default-with-listings
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx
    frame: KmpProfileScreen
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MProfile (alt KMP canvas)
useCases:
  - UC-47
  - UC-44
  - UC-45
  - UC-46
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header (with open user dropdown demo)
- [ ] [w] Portal header with notifications + avatar; clicking avatar opens user dropdown (Profil active · Obľúbené · Uložené hľadania · Dopyty · Odhlásiť)
- [ ] [m] Portal header collapsed; profile is accessed via bottom-nav

### Cover + identity card
- [ ] [w,m] Cover image + circular avatar (88px) overlaid · H1 name · email · "Verified ✓" badge · "Joined March 2024" sub
- [ ] [w] Right side: "Upraviť profil" secondary CTA

### Tabs
- [ ] [w,m] Moje inzeráty (default) · Aktivita · Hodnotenia · Nastavenia

### Tab · Moje inzeráty
- [ ] [w,m] H2 "Moje inzeráty" + count chip + "+ Pridať" CTA
- [ ] [w] Grid of listing-cards (3-up); each with status pill (Aktívny / Pozastavený / Predaný / Stiahnutý) + view count + inquiry count
- [ ] [m] Vertical card list

### Tab · Aktivita
- [ ] [w,m] Recent-activity timeline: listing published · inquiry received · favorite-saved · price-changed

### Right rail (≥1024px)
- [ ] [w] Stats card (Inzeráty · Obľúbené · Uložené hľadania · Dopyty counts) + "Verified ID" card with re-verify link

### Footer
- [ ] [w] Standard portal footer; [m] System bottom-nav

## States

- **Default**: as designed (3 listings, 6 activity entries, verified badge)
- **Empty (no listings)**: tab body shows empty card "Zatiaľ ste nepublikovali inzerát" + "+ Pridať prvý inzerát" primary CTA → reality/sell
- **Loading**: tab body skeleton (3 listing-card skels)
- **Error**: danger tile + retry; identity card preserved

## Notes

### Broader context

UC-47 portal user account hub. Pulls from UC-44 favorites, UC-45 saved searches, UC-46 inquiries, plus the user's own listings (if seller). Verified badge unlocks higher-trust display on listings.

### Specific (recent)

- Listings on this profile page are PUBLIC (visible to other users when they click on the user's name from a listing detail). Drafts and pending listings are hidden from public view.
- Tab "Hodnotenia" (Reviews) is for user-as-buyer reviews of agents — not yet wired; placeholder.
- Mobile profile has a different nav structure (bottom-nav with Profile tab vs. avatar dropdown on web) — keep visually consistent but functionally different.
- Re-verify ID flow links to a future identity-verification screen (out of scope).

## Agent Log

<!-- newest entries on top -->

- 2026-05-13 — agent: implemented KMP AccountScreen redesign per ui_kits/mobile-native/screens.jsx KmpProfileScreen. New layout: large-title top bar (Profil + Settings cog), centered hero (88dp gradient avatar with initials + name + Verified uppercase pill + email), 3-card stat strip (Favorites/Searches/Inquiries with tabular numerics + uppercase labels), section card grouping rows (Saved searches · 3 active, Comparison · 2 on list, Notifications with master toggle, Privacy & data, About · vX.X.X · Legal) with 40dp tile + chev/switch. Notification preferences detail card revealed via the master toggle (preserves existing 5 prefs: new listings, price drops, inquiry responses, listing updates, marketing). App settings + About cards restyled to match section-row idiom. Sign out as standalone danger-tinted card. Added 13 new strings (sk/en). buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: bootstrapped from bundle (pages/profile.html + mobile-new-pages.html MProfile frame); linked UC-47/44/45/46; relatedScreens to favorites + saved-searches + inquiries
