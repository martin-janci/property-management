# pm-security — 2026-06-24

## Headline

Three high-severity surfaces landed this window (Stripe Checkout, idempotency-key middleware, accounting payment-match) with no recorded security review. Cross-tenant IDOR class continues to surface in monolith routes; mitigation is the refactor program, which is now active.

## Findings

### pm-security-stripe-checkout-no-review [high]

**Finding:** Story 11.5 Stripe Checkout (#1726) is live but has no pm-security review record in .research/management/post-merge-review.json. Stripe webhooks + checkout-session creation + payment-reminder dispatch (#1709) all introduce signed-payload verification + amount-tampering surfaces that need explicit threat-model sign-off.

**Evidence:** PR #1726 (Story 11.5), #1709 (payment-reminders), #1716 (event-bus retry can re-emit payment events).

### pm-security-idempotency-key-replay [high]

**Finding:** Idempotency-key middleware #1688 mediates payment + state-mutating writes but its key-scope, TTL, and replay-after-failure semantics aren't documented. A too-wide scope (e.g. per-tenant instead of per-principal+route) lets one user replay another's request; too-narrow leaves payment double-write risk.

**Evidence:** PR #1688 idempotency-key middleware merged; no integration test referenced.

### pm-security-acc-payment-match-state-machine [high]

**Finding:** Accounting payment-match state machine (#1811) executes outside the api-server tenant context. If accounting-server reads/writes the shared DB without RLS connection, cross-tenant payment misallocation is possible at the new boundary.

**Evidence:** PR #1811 (accounting payment-match), #1808 (accounting-web), #1817 (TypeSpec spec).

### pm-security-ocr-endpoint-input-validation [medium]

**Finding:** OCR endpoint #1750 accepts arbitrary uploaded documents and likely shells out to a parser/external API. Needs explicit MIME/size guards, SSRF guard if it fetches signed URLs, and PII/log scrubbing on the parsed text path.

**Evidence:** PR #1750 OCR endpoint.

### pm-security-message-attachments-mime-confusion [medium]

**Finding:** Message attachments (#1702 backend, #1712 UI) introduce a new file-upload surface separate from documents. If MIME validation diverges from documents/core.rs allow-list, attackers can use messaging as an upload bypass for blocked types.

**Evidence:** PR #1702 + #1712 message attachments.

### pm-security-iot-realtime-authz [medium]

**Finding:** Real-time IoT dashboard (#1685) + IoT alert error feedback (#1740) likely push device-state events over WebSocket. WS authz must use the post-#480 token-not-in-query pattern and tenant-scope filter every fan-out event — broadcasting across tenants is the classic IoT-dashboard failure mode.

**Evidence:** PR #1685, #1740. risks.json carries pm-qa-jwt-token-in-ws-logs open (#480) — same WS plumbing.

### pm-security-residual-audit-hash-debug [low]

**Finding:** Residual P1-04 from PR #435 (Debug-format audit-hash) still open in risks.json. None of the 95 merged PRs touch it.

**Evidence:** risks.json pm-security-audit-hash-debug-format-p1-04 status=open since 2026-05-25.

## Next actions

- [high] Threat-model + post-merge security pass on Stripe Checkout #1726 (webhook signature verification, amount tampering, session-id leakage)
- [high] Document idempotency-key scope/TTL/replay contract for middleware #1688; add integration test pinning scope = per-principal+route+body-hash
- [high] Verify accounting-server uses the RLS-aware DB connection (or equivalent) before #1821 umbrella merges; cross-tenant payment-match regression test
- [medium] Audit message-attachments MIME allow-list parity vs documents/core.rs; consolidate to one shared validator
- [medium] Audit IoT WebSocket fan-out for tenant-scoped filtering + token-not-in-query (re-use post-#480 pattern)
- [low] Close residual P1-04 Debug-format audit-hash from PR #435

## Risks

- **Stripe Checkout webhook signature bypass — if STRIPE_WEBHOOK_SECRET is missing/empty in env, the verifier may silently accept unsigned payloads (common pattern)** (medium/high) — Fail-closed assertion at boot: server refuses to start if STRIPE_WEBHOOK_SECRET empty in prod profile; integration test with tampered signature returns 400
- **Accounting-server tenant boundary leak — new server reading shared DB without RLS lets one tenant's payment match overwrite another's ledger** (medium/high) — Mandate RLS connection in accounting-server DB layer; add cross-tenant payment-match regression test before #1821 merges
- **Idempotency-key scope mis-configuration leads to either replay-attack window or payment double-write (depending on direction of error)** (medium/high) — Document the contract; pair test fixtures (one for replay, one for double-write); deny merge of payment routes that don't opt-in
- **Message-attachments upload becomes a MIME-bypass vector for the document allow-list** (medium/medium) — Single shared validator; CI grep that flags any handler accepting multipart without calling the validator
- **IoT WS broadcast cross-tenant leak — IoT alert events fan out to subscribers without tenant_id filter** (low/high) — Subscriber registration includes tenant_id; fan-out predicate asserts match; regression test with two-tenant fixture
