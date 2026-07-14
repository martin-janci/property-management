---
id: ppt/document-sign
name: Document Signing (Signer)
product: ppt
implementations:
  ppt-web:
    route: /sign
    component: DocumentSignPage
    buildStatus: planned
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/document-detail
    rel: parent
  - id: ppt/documents
    rel: sibling
sharedComponents:
  - pdf-preview
useCases:
  - UC-22.7
epics:
  - Epic-84
owner: pm-integration
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

Signer-facing public page opened from the emailed `{BASE_URL}/sign?token=<v1…>`
link (E-Signature Email Integration, Epic 84.2). No auth — the `v1.` HMAC token
is the only credential. This screen is the frontend consumer for the
already-shipped backend `/api/v1/signatures/sign` endpoints; the ppt-web page
itself is not built yet (`buildStatus: planned`).

### Load / render context (`GET /api/v1/signatures/sign?token=…`)
- [ ] [w] Lift `token` from the query string and call `getSignContext`
- [ ] [w] Show document subject/title (`subject`) and the requester's optional `message`
- [ ] [w] Show signer identity — `signer_name` + `signer_email` (echoed from the verified token)
- [ ] [w] Render the document to sign (PDF preview surface)
- [ ] [w] Reflect `signer_status` (`pending` / `viewed`) so a returning signer sees their state

### Sign / decline (`POST /api/v1/signatures/sign?token=…`)
- [ ] [w] "Adopt & sign" flow — capture `typed_name` as lightweight signing evidence
- [ ] [w] On success, show confirmation from `SubmitSignatureResponse.message`; branch on `status` (`in_progress` while other signers remain vs `completed` once everyone has signed)
- [ ] [w] Guard against re-sign — once the signer is `signed`/`declined` the endpoint refuses; surface a friendly "already signed" state instead of an error toast

## States

- **Empty**: n/a — the page always resolves against a single token.
- **Loading**: fetching render context after lifting the token.
- **Error**:
  - `SIGNING_LINK_EXPIRED` (HTTP 410) — token past expiry; prompt "Ask the sender for a new one".
  - `INVALID_SIGNING_LINK` (HTTP 401) — bad/unknown token; generic message (backend deliberately does not distinguish bad-MAC from unknown-request).
  - Already-signed / superseded (HTTP 409 / 410 on POST) — signer already completed or the request moved on.

## Notes

### Broader context

This is the public consumer half of the e-signature feature. The manager-side
half — creating a signature request, listing signer status, reminders, and
cancel — lives on **`ppt/document-detail`** via `DocumentSignaturePanel`
(Story 7B.3) and is already shipped. The emailed link brings an external signer
here.

Backend is complete and tested: `get_sign_context` / `submit_signature`
(`backend/servers/api-server/src/routes/signatures.rs`, `public_sign_router`
mounted at `/api/v1/signatures`, no auth extractor), covered by
`esignature_sign_consumer_tests.rs` (happy path + tamper/replay +
CONFLICT/GONE). The signing URL is built by the ESIGN provider as
`{BASE_URL}/sign?token=<v1…>` (`BASE_URL` env, default `http://localhost:3000`).

The `/api/v1/signatures/sign` endpoints are **not** registered in `@ppt/sitemap`,
so they are intentionally omitted from the `endpoints:` frontmatter list
(the validator errors on unknown endpoint ids) and documented here in prose.
No `sitemapRefs` for the same reason — `/sign` is not in the sitemap route table.

### Specific (recent)

- ppt-web page is not implemented yet; `route: /sign` records the intended
  public path. When the page is built, wire it as an unauthenticated route and
  flip `ppt-web.buildStatus` → `shipped`.
- Mobile is `n/a` — the signing link is a public web page opened from email, not
  a screen in the RN app.

## Agent Log

<!-- newest entries on top -->

- 2026-07-14 — agent: gap-84-2-no-screen-map-entry-for-a — created this screen-map for the signer-facing `/sign` page (Epic 84.2 E-Signature Email Integration). Backend `/api/v1/signatures/sign` consumer endpoints are shipped + tested; frontend page is `planned`. Endpoints/sitemapRefs left empty by design (signature routes are not in `@ppt/sitemap`); documented in prose. Manager-side request creation stays on `ppt/document-detail`.
