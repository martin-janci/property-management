---
id: ppt/article-detail
name: Article Detail
product: ppt
sitemapRefs:
  ppt-web: ppt-news-detail
implementations:
  ppt-web:
    component: ArticleDetailPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - news_get
relatedScreens:
  - id: ppt/news
    rel: parent
epics:
  - Epic-59
sharedComponents:
  - rich-text-renderer
  - rich-text-editor
  - table-of-contents
  - thread
  - status-pill
  - radio-cards
  - file-upload
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-article-detail.html
    frame: read-mode-published-7-comments / edit-mode-auto-saved / draft-preview / loading
useCases:
  - UC-13
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Reader chrome (Read mode)
- [ ] [w] Manager chrome + breadcrumb `Aktuality / <Category> / <Title>`
- [ ] [w] Category eyebrow + H1 (max 90 chars, wraps) + author byline (avatar + name + role + publish date + read-time)
- [ ] [w] Status pill row: Publikované + audience pill (Všetci/Vlastníci/Manageri) + optional Pripnuté
- [ ] [w] State-aware action toolbar:
  - Published: Upraviť · Zdieľať · Pripnúť/Odopnúť · Archivovať (danger ghost)
  - Draft: Náhľad · Publikovať primary
  - Scheduled: countdown chip + Upraviť + Zrušiť plánovanie

### Hero image
- [ ] [w] Full-width 16:9 max 1024 centered + caption + photo credit muted

### 2-col body (max 1280; article 720 + rail 320)
- [ ] [w] Article column: paragraphs · h2/h3 · pull-quote (left bar accent, italic 18px) · bulleted/numbered lists · inline image · embed card · code-fenced "fact box"
- [ ] [w] Tag chips at body footer

### Right rail · sticky (top: 84px)
- [ ] [w] **Obsah** (table of contents, auto-from h2, current section brand-600)
- [ ] [w] **Zdieľať** (Copy link · Email · WhatsApp · QR)
- [ ] [w] **Súvisiace dokumenty** (2–3 file thumbs)
- [ ] [w] **Súvisiace oznamy** (1–2 announcement minis)
- [ ] [w] **Štatistiky čítania** (manager-only): Zobrazenia · Unikátni · Avg čas · Komentárov

### Comments section (below body)
- [ ] [w] Header "Komentáre · 7" + sort dropdown
- [ ] [w] Composer: textarea + attach + emoji + markdown-preview toggle + "iba pre managera" toggle
- [ ] [w] Threaded replies (max 1 level deep; deeper collapsed); reactions (👍 ❤️ 🚩); manager bubbles violet-tinted
- [ ] [w] Manager moderation toolbar (per row): Skryť · Pripnúť · Označiť · Vymazať

### Edit mode (toggled by Upraviť)
- [ ] [w] Body becomes rich-text editor with floating toolbar (B/I/link/list/heading/quote/image/embed) + slash-command menu
- [ ] [w] Auto-save indicator above toolbar
- [ ] [w] Right rail switches to: Publikovanie (audience radio-cards + notification segmented + schedule date) · Tagy + kategória · Cover image dropzone · SEO (collapsible)

## States

- **Loaded · Published with 7 comments**
- **Edit mode** — full editor with publishing rail
- **Draft preview** — read-only with "Náhľad konceptu" warning banner
- **Loading** — hero skeleton + 4 paragraph skels + rail skels

## Notes

### Broader context

UC-13 long-form article surface. Edit mode is a substantial separate UX from read mode but lives on the same screen-map (single component, mode toggle).

### Specific (recent)

- Rich-text storage format: Markdown / Lexical / TipTap-JSON / HTML — engineering decision affects round-trip block types. Flag at handoff.
- Slash-command menu requires keyboard handling (Tab/Arrow/Enter); ensure focus-trap when active.
- Comments threading: max 1 nested level; deeper collapsed to "+ N skrytých odpovedí" expander.
- SEO card only relevant if articles are public; gate via feature flag.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: integrated Batch D (pages/ppt-article-detail.html — 4 artboards: read-mode / edit-mode / draft-preview / loading); flipped redesignStatus → in-progress; attached designSource; populated 7 sections + 4 states + 4 notes; declared 7 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
