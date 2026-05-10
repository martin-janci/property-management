---
id: ppt/news
name: News
product: ppt
sitemapRefs:
  ppt-web: ppt-news
implementations:
  ppt-web:
    component: NewsListPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - news_list
relatedScreens:
  - id: ppt/article-detail
    rel: child
  - id: ppt/home
    rel: parent
epics:
  - Epic-59
sharedComponents:
  - article-card
  - chip-group
  - search-bar
  - bulk-action-bar
  - sort-dropdown
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-news.html
    frame: loaded-grid-3up-1featured-1pinned-1scheduled-1draft / empty / loading-6-skel / error-503
useCases:
  - UC-13
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with `Aktuality` tab active (or sub-nav under Oznamy — flag IA decision)
- [ ] [w] Breadcrumb `Aktuality / Všetko`

### Page header
- [ ] [w] H1 "Aktuality" + count chip "84 článkov · 2 koncepty · 14 plánovaných"
- [ ] [w] Right toolbar: sort dropdown ("Najnovšie ↓") + "+ Nový článok" primary

### Toolbar
- [ ] [w] Segmented category chips with counts: Všetko · 84 / Schôdze · 12 / Odstávky · 18 / Susedské · 22 / Mesto · 14 / Iné · 18
- [ ] [w] Search "Hľadať v názvoch a tele článkov…"
- [ ] [w] View-mode toggle (List / Mriežka), default Mriežka

### Card grid (3-up at ≥1024, 2-up 768-1023, 1-up <768)
- [ ] [w] Featured story spans 2 columns at top of grid (1 per page)
- [ ] [w] Each card: 16:9 cover OR colored gradient · top-left badges (Plánované 22.5. / Koncept / Pripnuté) · category eyebrow · H3 title (2-line clamp) · 2-line excerpt · author + read-time + date + comment count

### Bulk-action bar
- [ ] [w] Visible when ≥1 selected: Pripnúť · Zrušiť pripnutie · Plánovať na… · Archivovať

### Right rail (≥1280, optional)
- [ ] [w] "Najčítanejšie tento týždeň" top-5 + "Plánované" + "Koncepty" mini-rows

## States

- **Loaded**: 9-card grid (1 featured 2-col + 8 regular), 1 pinned, 1 scheduled, 1 draft
- **Empty**: newspaper-icon tile + "Zatiaľ žiadne aktuality" + body + primary "+ Napísať prvý článok"
- **Loading**: 6 skeleton cards
- **Error 503**: danger tile + retry; toolbar interactive

## Notes

### Broader context

UC-13 long-form publishing. Distinct from `ppt/announcements` — articles are editorial/educational; announcements are operational/imperative. IA decision needed: separate top-level tab vs. sub-nav under Oznamy.

### Specific (recent)

- Featured 2-col card breakpoints: stays 2-col on 768-1023 (forces 1 featured + 1 regular row), reduces to 1-col below 768.
- Cover-less cards use colored gradient tile per category (consistent palette).
- Bulk-pin clears at 0 selection (slide-out animation respects reduced-motion).

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: integrated Batch D (pages/ppt-news.html — 4 artboards: loaded-grid / empty / loading / error); flipped redesignStatus → in-progress; attached designSource; populated 6 sections + 4 states + 3 notes; declared 5 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
