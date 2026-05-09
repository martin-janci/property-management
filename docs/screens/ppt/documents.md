---
id: ppt/documents
name: Documents
product: ppt
sitemapRefs:
  ppt-web: ppt-documents
  mobile: mobile-documents
implementations:
  ppt-web:
    component: DocumentsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
  mobile:
    component: DocumentsScreen
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - documents_list
relatedScreens:
  - id: ppt/document-detail
    rel: child
  - id: ppt/upload-document
    rel: child
epics:
  - Epic-39
  - Epic-7
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

Document management dashboard Building documents

### Specific (recent)

## Agent Log

<!-- newest entries on top -->

- 2026-05-08 — init: created from scan (source: sitemap)
