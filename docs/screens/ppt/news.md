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
    redesignStatus: not-started
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
epics:
  - Epic-59
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->
- [ ] [w,m] (none yet)

## States

- **Empty**:
- **Loading**:
- **Error**:

## Notes

### Broader context

News and announcements list

### Specific (recent)

## Agent Log

<!-- newest entries on top -->

- 2026-05-08 — init: created from scan (source: sitemap)
