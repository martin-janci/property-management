---
id: ppt/document-sign
name: Document Signing (Signer)
product: ppt
implementations:
  ppt-web:
    route: /sign
    component: DocumentSignPage
    buildStatus: shipped
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
already-shipped backend `/api/v1/signatures/sign` endpoints. The ppt-web page
is built and wired as a public (unauthenticated) `/sign` route
(`buildStatus: shipped`).

### Load / render context (`GET /api/v1/signatures/sign?token=…`)
- [x] [w] Lift `token` from the query string and call `getSignContext`
- [x] [w] Show document subject/title (`subject`) and the requester's optional `message`
- [x] [w] Show signer identity — `signer_name` + `signer_email` (echoed from the verified token)
- [ ] [w] Render the document to sign (PDF preview surface) — deferred: the public render context returns no document URL (document fetch is auth-gated), so no PDF is shown yet; only document metadata is surfaced
- [x] [w] Reflect `signer_status` (`pending` / `viewed`) so a returning signer sees their state

### Sign / decline (`POST /api/v1/signatures/sign?token=…`)
- [x] [w] "Adopt & sign" flow — capture `typed_name` as lightweight signing evidence
- [x] [w] On success, show confirmation from `SubmitSignatureResponse.message`; branch on `status` (`in_progress` while other signers remain vs `completed` once everyone has signed)
- [x] [w] Guard against re-sign — once the signer is `signed`/`declined` the endpoint refuses; surface a friendly "already signed" state instead of an error toast

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

- ppt-web page is now built: `DocumentSignPage`
  (`features/documents/pages/DocumentSignPage.tsx`) wired as a PUBLIC `/sign`
  route in `routes/groups/documents.tsx` (no `ProtectedRoute`). It consumes the
  new `@ppt/api-client` signer hooks `useSignContext` / `useSubmitSignature`
  (auth-less `fetch`, typed `SignError` carrying the backend error `code`).
  Strings live under the `documentSign.*` i18n namespace in all six locale
  bundles (parity locked by `documentSignI18n.test.ts`).
- PDF preview is intentionally deferred — the public render context carries no
  document URL (the document blob is behind an authenticated endpoint), so the
  page shows the document subject/message metadata rather than the file itself.
  Revisit if a token-scoped public document-fetch endpoint is added.
- Mobile is `n/a` — the signing link is a public web page opened from email, not
  a screen in the RN app.

## Agent Log

<!-- newest entries on top -->

- 2026-07-15 — agent: gap-84-2-build-document-sign-page — built the signer-facing `/sign` page (`DocumentSignPage`) against the shipped `/api/v1/signatures/sign` API; added public signer client (`getSignContext`/`submitSignature`/`SignError` + `useSignContext`/`useSubmitSignature`) to `@ppt/api-client` esignature module; added `documentSign.*` strings to all 6 locales + a key-parity regression test; flipped `ppt-web.buildStatus` planned→shipped. PDF preview deferred (no public document URL in render context).
- 2026-07-14 — agent: gap-84-2-no-screen-map-entry-for-a — created this screen-map for the signer-facing `/sign` page (Epic 84.2 E-Signature Email Integration). Backend `/api/v1/signatures/sign` consumer endpoints are shipped + tested; frontend page is `planned`. Endpoints/sitemapRefs left empty by design (signature routes are not in `@ppt/sitemap`); documented in prose. Manager-side request creation stays on `ppt/document-detail`.
