---
id: ppt/document-templates
name: Document Templates — List & Generate
product: ppt
implementations:
  ppt-web:
    route: "/documents/templates"
    component: DocumentTemplatesPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/documents
    rel: parent
  - id: ppt/document-detail
    rel: child
sharedComponents:
  - GenerateDocumentDialog
diagrams: []
useCases: []
epics:
  - Epic-7B
designSources: []
owner: pm-frontend
---

# Document Templates — List & Generate

The template-driven document generation surface for Epic 7B (Story 7B.2). It
lets a manager browse reusable document templates and run the
select → context → prefill → preview → save flow that produces a real document
in the org's library.

Mounted by `routes/groups/documents.tsx` on the hand-written `@ppt/api-client`
`templates` module (`useTemplates`, `useTemplate`, `useGenerateDocument`). These
endpoints are **not** `@ppt/sitemap`-registered and **not** TypeSpec-generated,
so `endpoints: []` above and the contract is documented in prose below.

## Routes

- `/documents/templates` — `DocumentTemplatesPage`: searchable, type-filterable
  list of the organization's document templates. Each row opens the generate
  dialog. Reachable from the Documents page header ("Šablóny" link).

## Endpoints (prose — not sitemap-registered)

All under `/api/v1` and tenant-scoped server-side (manager role required for
mutations; generation is allowed for the caller's organization templates):

- `GET /templates` — list (`template_type`, `search`, `limit`, `offset` query
  params) → `{ templates: TemplateSummary[], count, total }`.
- `GET /templates/{id}` — single template with placeholders + content
  (`TemplateWithDetails`).
- `POST /templates/{id}/generate` — body
  `{ values: Record<string,string>, title, description?, category, folder_id? }`
  → `{ document_id, message }`. Server validates required placeholders
  (400 `MISSING_PLACEHOLDERS`) and the category (400) before creating the doc.
- CRUD (`POST /templates`, `PUT /templates/{id}`, `DELETE /templates/{id}`) is
  also exposed in the api-client module; no authoring UI ships in this story.

## Functionality Checklist

### List page (`/documents/templates`)
- [x] [w] H1 "Šablóny dokumentov" + subtitle, back-link to `/documents`
- [x] [w] Search input (debounced via React Query key) + template-type filter select
- [x] [w] Template cards: name + type badge + description + placeholder/usage counts + "Generovať" CTA
- [x] [w] Loading skeleton rows
- [x] [w] Error state with retry
- [x] [w] Empty state (distinguishes "no templates" vs "no matches for filters")

### Generate dialog (`GenerateDocumentDialog`)
- [x] [w] Loads the template (`useTemplate`); loading + load-error (retry) states
- [x] [w] Context selector: building → unit; pre-fills placeholders whose name
  matches building/unit fields (building name/address, unit designation/floor)
- [x] [w] Typed placeholder inputs (text/date/number/currency) seeded with defaults
- [x] [w] Document metadata: title (required), category (backend `document_category`), description
- [x] [w] Live preview mirroring the server's `{{name}}` substitution
- [x] [w] Required-placeholder validation blocks submit + lists missing names
- [x] [w] Generate failure surfaces the server message inline
- [x] [w] On success → navigate to `/documents/{document_id}`
- [x] [w] Focus trap + Escape close (WCAG 2.1.2), overlay click closes

## States

- **List · loading**: 4 shimmer skeleton cards.
- **List · error**: danger tile + retry.
- **List · empty**: "no templates yet" or "no matches" depending on active filters.
- **List · loaded**: template cards with type badge + counts.
- **Dialog · loading/error**: spinner copy / retry on the template fetch.
- **Dialog · ready**: context + placeholders + metadata + live preview.
- **Dialog · invalid**: required placeholders highlighted, alert lists names.
- **Dialog · submitting**: primary button shows "Generuje sa…", disabled.

## Notes

### Broader context

Story 7B.2 closes the last genuinely-new frontend gap from GH epic #974
(document templates). The backend (`routes/templates.rs`,
`services/document_generation.rs`, migration `00036_add_document_templates.sql`)
shipped CRUD + generate; this surface is the first UI to consume it.

### Specific

- The building/unit **context** is a frontend convenience only — the backend
  `generate` endpoint takes a flat `values` map, so context selection just
  pre-fills matching placeholder inputs (by case-insensitive name match,
  EN + SK aliases) and never overwrites a value the user already typed.
- Only **non-empty** values are sent; the server applies each placeholder's
  `default_value` for omitted keys, matching `DocumentTemplate::generate_content`.
- i18n keys live under the `documentTemplates` namespace (en/sk/cs; de/pl/hu
  fall back to en).
- The generated document is created as `text/markdown` server-side; this UI
  hands off to `ppt/document-detail` after generation.

## Agent Log

<!-- newest entries on top -->

- 2026-06-22 — agent (FrontendEngineer): BIT-207 — shipped `@ppt/api-client`
  `templates` module (hand-written, mirrors `esignature`) + `DocumentTemplatesPage`
  and `GenerateDocumentDialog` on `/documents/templates`; wired into App router +
  Documents header; en/sk/cs i18n; vitest coverage for preview/validation/submit.
