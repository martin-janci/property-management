---
id: ppt/financial-invoices
name: Invoice Management
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/financial/invoices"
    component: InvoiceManagementPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/financial
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-11
designSources: []
owner: pm-frontend
---

# Invoice Management

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-04 — **create + detail sub-routes now registered.** `/financial/invoices/new`
  (`InvoiceForm` → `createInvoice`, unit picker fed by `useBuildings` + per-building
  `/buildings/{id}/units`) and `/financial/invoices/:invoiceId` (`InvoiceDetail` →
  `getInvoice`, with `sendInvoice` / `downloadInvoicePdf` / "Record payment" →
  `/financial/payments/new?unitId=`) live in `routes/groups/financial-detail.tsx`,
  registered from `AppRoutes`, manager-gated (`MANAGER_ROLES`). Previously the
  "New invoice" button and invoice-row clicks fell through to not-found. Form /
  detail copy is now i18n'd (`financial.invoiceForm.*`, `financial.invoiceDetail.*`).
  Test: `routes/groups/financial-detail.route.test.tsx`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:526`.

## Agent Log
- 2026-11-05 — agent: screen-map-drift-pr-3242-ppt — reconcile i18n drift from PR #3242 (`fix(ppt-web): localize financial, buildings and my-unit pages`, merged 2026-11-04). The list/detail surfaces (`InvoiceList`/`InvoiceDetail`/`InvoiceForm`/`InvoiceStatusChart`/`OverdueInvoicesList`) moved their remaining hardcoded English into `financial.*` i18n keys across en/sk/cs/de/pl/hu; invoice-status enum maps to keys with a raw-value fallback; money/dates format via `i18n.language`. No frontmatter change — docs-only.
- 2026-05-18 — agent: created stub for unmapped route.
- 2026-06-03 — agent: confirmed InvoiceManagementPageRoute wired to listInvoices (status + pagination) + sendInvoice via TanStack Query (#975.2); apiStatus -> partial (no building_id/search params).
- 2026-11-04 — agent: registered `/financial/invoices/new` + `/financial/invoices/:invoiceId` (attached to this screen-map; no separate maps) and i18n'd InvoiceForm/InvoiceDetail.

## Agent Log

<!-- newest entries on top -->

- 2026-11-04 — agent: registered /financial/invoices/:invoiceId — deep-links the InvoiceDetail modal (getInvoice) over the list; /financial/invoices/new still has no create page
