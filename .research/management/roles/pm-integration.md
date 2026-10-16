# pm-integration (2026-10-16)

**Summary.** The sprint-status.yaml sprint (Epics 6/7A/8A/10A) is stale against what is actually shipping. The real integration risk is contract drift: the analytics repositories and migrations disagree on schema (#3104, draft PR #3107), and the KMP inquiries client cannot parse reality-server responses. The sprint file wasn't re-read after its 2026-07 reconciliations, and I did not open the TypeSpec / OpenAPI / generated-client files, so drift between those layers is unchecked.

## Next actions

1. **[high]** Land PR #3107 (portfolio_benchmarks reconcile) together with the other #3104 items: remove the `name` column from the `property_comparisons` SELECT, fix the portfolio lookup, and fix `get_dashboard_summary`. Do this with add-column migrations (00236 / 00237 / 00238) and un-ignore `pa_list_comparisons_succeeds`, `pp_list_comparisons_succeeds`, `analytics_owner_success_tests`. DoD: all #3104 bullets fixed, ignored tests un-ignored and green on `#[sqlx::test]`, no BIT-622 refs remain. — `pm-backend`.
2. **[high]** Fix the KMP inquiries contract. Rename `pageSize` to `limit` as an optional field with a default. Make `page` and `limit` optional so the realtors shape `{inquiries, total}` parses. Send `limit` instead of `page_size` in `InquiryRepository`. DoD: Kotlin unit test decodes both real server payloads without `MissingFieldException`, and the query param matches `InquiryListQuery` in `inquiries.rs`. — `pm-mobile`.
3. **[high]** Make the OpenAPI spec the source of truth for the reality-server inquiries responses. Check that TypeSpec, utoipa annotations, `@ppt/reality-api-client` and the KMP `openapi-generator` output all agree on `limit` vs `page_size` and on the realtors response shape. Add a CI contract check on `api-validation.yml`. DoD: one canonical paging shape per endpoint, generated clients regenerated, and `api-validation.yml` fails on drift. — `pm-backend`.
4. **[medium]** Add a schema-drift guard for repositories. Run a CI test that executes each repository's `list` / summary query against the migrated schema (smoke test per repo), so a SELECT on a missing column fails in CI rather than as a runtime 500. DoD: analytics repos (comparisons, portfolio performance, owner-analytics) have non-ignored success-path tests in CI. — `pm-qa`.
5. **[medium]** Resolve `create-listing-not-wired` in mobile-native. Either wire it to the reality-server create-listing endpoint, or remove the entry point and mark it unsupported. Depends on SSO + auth contract. DoD: the `create-listing` action calls a real endpoint, or it is hidden and the backlog item is closed. — `pm-mobile`.
6. **[medium]** Audit outbound HTTP callers after the NAT64 SSRF fix in #3103. Check that e-signature, webhook, S3 and embedding integrations all route through the filtered client, and that timeouts / retries / error mapping behave consistently when a destination is blocked. DoD: a list of outbound call sites showing which use the SSRF-filtered client, plus a test that a blocked destination returns a mapped 4xx/5xx and not a panic or hang. — `pm-security`.

## Risks

- **High / high.** Repository SQL has drifted from migrations (missing columns such as `property_comparisons.name`, `portfolio_benchmarks`), so analytics endpoints return 500 in production while the success tests are ignored. OpenAPI promises endpoints that fail at runtime. Mitigate by merging #3107 + rest of #3104 with un-ignored tests, plus repo smoke-test guard.
- **High / high.** The KMP inquiries client throws `MissingFieldException` on every real response, so inquiries listing in the Reality mobile app (and the realtor flow) is broken. iOS inquiries story is marked done (82-5) with no dedicated test, which hides the failure. Mitigate by fixing the model, adding contract tests with fixture payloads taken from reality-server, and reopening/annotating 82-5.
- **Medium / medium.** No single owner for paging conventions. reality-server uses `limit`, KMP sends `page_size`, the realtors endpoint omits both. Hand-written Kotlin models bypass the generated SDK and drift silently. Mitigate by standardising the paging envelope in TypeSpec and generating KMP models from it; ban hand-written DTOs for contract types.
- **High / medium.** `sprint-status.yaml` is badly stale. Dependency tracking against the real active work is unreliable. Refresh the sprint file; add the analytics and mobile-native contract items as tracked stories.
- **Medium / high.** Test-hardening batch item #480 (WebSocket auth token in query param, logged and not re-validated after JWT expiry) still appears open in the YAML although 8a-3 is done. If truly open, a token-in-URL leak remains at the realtime integration boundary.

## Open questions

- Is #480 (WebSocket token in query param) actually closed? YAML lists it as open while 8a-3 is marked done.
- Is the TypeSpec the real source for reality-server routes, or are they utoipa-only? That decides where the paging fix belongs.
- Is the inquiries response mismatch present in the generated KMP client as well, or only in the hand-written `InquiryModels.kt`?
- Does #3107 cover all four #3104 bullets, or only portfolio_benchmarks? The migration list in the draft was not verified.
- Which endpoint does mobile create-listing target, and does it need the api-server SSO token or a reality-server session?

## Decisions needed

- Canonical paging envelope (`limit`/`page`/`total` vs `page_size`) across api-server and reality-server, plus the realtors response shape — owner: `pm-architect`.
- Fix the KMP client to match the server, or change the server to match the client. Recommendation: fix the client, since the backend contract is already consumed by web — owner: `pm-mobile`.
- Reopen or annotate story 82-5 (inquiries done but unverified) given the contract breakage — owner: `pm-scrum-master`.
