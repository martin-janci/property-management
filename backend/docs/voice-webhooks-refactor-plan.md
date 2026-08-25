# Refactor Plan — `routes/voice_webhooks.rs`

**Target:** `backend/servers/api-server/src/routes/voice_webhooks.rs`
**Status:** proposed (planning deliverable — no code change in this PR)
**Owner:** pm-tech-lead
**Date:** 2026-08-25
**Context:** recurring churn hotspot (3 hotspot windows). PR #2838 centralized
voice OAuth **token encryption** (`encrypt_voice_token_pair`), but the
scheduler (rate-limiter), OAuth, and token-refresh paths still cluster in one
flat 2896-line module.

---

## 1. Current state

| Metric | Value |
| --- | --- |
| Total lines | 2896 |
| Non-test code | ~1851 (lines 1–1851) |
| Tests | ~1030 (lines 1868–2896) |
| Functions | 96 |
| Structs | 6 |
| Module shape | single flat file, no submodules |

The file is one flat module that mixes seven distinct concerns. It is an active
churn hotspot (recent: #2838 token encryption, #2834 quiet-hours drain), so its
size directly amplifies merge-conflict cost and review load for every touch.

**External surface (compatibility budget):** the *only* symbol referenced from
outside the file is `voice_webhook_router()` (used at `lib.rs:341`). The
handlers carry their own `#[utoipa::path]` macros and are **not** aggregated
into a central `#[openapi(paths(...))]` list by module path (grep confirms no
external reference to `oauth_token_exchange` / `alexa_webhook` / etc.). This
means any decomposition that keeps `voice_webhook_router()` at
`routes::voice_webhooks::voice_webhook_router` is call-site-invisible.

---

## 2. Clusters (where the mass is)

| # | Cluster | Lines (approx) | Character |
| --- | --- | --- | --- |
| C1 | Token crypto helpers | 52–103 | small, cohesive; **already centralized by #2838** |
| C2 | Rate-limiter "scheduler" | 105–156 | **two near-verbatim duplicated blocks** (refresh + exchange) |
| C3 | Router | 163–175 | trivial |
| C4 | Webhook handlers (Alexa + Google) | 198–404 | ingest → verify → authenticate → process → build response |
| C5 | **OAuth handlers (exchange + refresh)** | 433–916 | the two largest fns (235 + 218 lines); dense shared shape |
| C6 | Signature verification | 940–1508 | Alexa cert-chain + Google JWKS + HMAC; self-contained crypto |
| C7 | Voice user authentication | 1552–1709 | indexed lookup + decrypt-scan; **security-critical** |
| C8 | Response builders | 1712–1849 | pure, no I/O |
| C9 | Tests | 1868–2896 | ~1030 lines incl. embedded PEM fixtures + `#[sqlx::test]` |

### Duplication / entanglement found

- **D1 — rate limiters (C2).** `VOICE_REFRESH_*` and `VOICE_EXCHANGE_*` are two
  copies of the same idiom: 3 consts + 1 `LazyLock` static + 1 `*_attempt_allowed`
  fn, identical except the name. ~40 duplicated lines. They must stay **two
  independent maps** (test `exchange_rate_limiter_isolates_per_user` pins this).
- **D2 — OAuth token-minting ladder (C5).** `oauth_token_exchange` and
  `oauth_token_refresh` both run the same branch:
  `has_platform(real exchange/refresh)` → `encrypt_voice_token_pair` → tuple;
  `else if debug` → simulated token; `else` → release fail-closed 503 (#890).
  This ladder is duplicated across the two handlers.
- **D3 — `VoiceCommandProcessor::new(...)`** is constructed inline **3×**
  (Alexa launch, Alexa intent, Google) with the same five repo clones.
- **D4 — error tuple.** `(StatusCode, Json(ErrorResponse::new(code, msg)))` is
  spelled out by hand dozens of times. Sibling `routes/auth/mod.rs` already
  solved exactly this with an `AuthError` type alias + `err_response()` helper.

---

## 3. Proposed decomposition

Convert the flat file into a **module directory** `routes/voice_webhooks/`,
following the existing precedent `routes/auth/` (which is `mod.rs` +
`password_reset.rs` + `registration.rs` + `sessions.rs`, submodules using
`use super::*` and a shared `err_response`). Keep `voice_webhook_router()` in
`mod.rs` so the public path is unchanged and `lib.rs:341` needs no edit.

```
routes/voice_webhooks/
├── mod.rs          # router + `VoiceError` alias + `err_response()` + re-exports
├── crypto.rs       # C1: encrypt_voice_token_pair, voice_encryption_required,
│                   #     voice_access_token_hash, voice_simulated_oauth_allowed
├── rate_limit.rs   # C2 collapsed: ONE generic per-user limiter helper,
│                   #     two distinct static maps (exchange / refresh)
├── oauth.rs        # C5: exchange + refresh, sharing a `mint_encrypted_tokens`
│                   #     helper that factors the D2 has_platform/simulated/
│                   #     fail-closed ladder
├── handlers.rs     # C4: alexa_webhook, google_actions_webhook, health,
│                   #     + `process_voice_command()` folding D3
├── signature/
│   ├── mod.rs      # verify_webhook_signature dispatch + HMAC helpers
│   ├── alexa.rs    # cert cache, http client, fetch/validate/verify cert+sig
│   └── google.rs   # JWKS cache, decoding key, verify_google_id_token
├── auth.rs         # C7: authenticate_voice_user, voice_device_token_matches
└── response.rs     # C8: alexa/google response + link-account builders,
                    #     extract_alexa_command_text
```

Each submodule keeps its own `#[cfg(test)] mod tests`; the C9 test block is
split so tests travel **with** the code they cover (the PEM fixtures go to
`signature/`, the `#[sqlx::test]` pool test to `auth.rs`).

### Dedup wins folded in (each independently valuable)

- **W1** — collapse D1 into one generic limiter helper (removes ~40 lines).
- **W2** — extract D2 ladder into `mint_encrypted_tokens(...)`.
- **W3** — extract D3 into `process_voice_command(...)`.
- **W4** — introduce `VoiceError` + `err_response()` (mirror `auth/mod.rs`).

---

## 4. Risks

| # | Risk | Severity | Mitigation |
| --- | --- | --- | --- |
| R1 | C6/C7 are **fail-closed security code** (#2658 horizontal-priv-escalation, #890 fail-open, #2769 amplification, #765 fail-closed encryption). A refactor slip re-opens a CVE-class bug. | High | Treat every move as a **pure code-move**: move the covering test in the same commit; reviewer diffs *behavior*, not just compile. No logic edits inside a move commit. |
| R2 | Merging the two rate limiters (W1) could accidentally share one map, breaking exchange/refresh independence. | Med | Keep **two distinct statics**; the generic helper takes the static by reference. `exchange_rate_limiter_isolates_per_user` must stay green. |
| R3 | `#[utoipa::path]` macros must stay attached and OpenAPI generation must still resolve. | Low | Verified: no external `paths(...)` registration references these handlers by path; only `voice_webhook_router()` is referenced externally. Keep macros on the handlers; run the api-validation build. |
| R4 | Tests embed PEM cert/key constants + `#[sqlx::test]` pool tests; moving them can break sqlx offline data / fixtures. | Med | Move fixtures with their tests; re-run `cargo test -p api-server` and `cargo sqlx prepare` before the PR. |
| R5 | `use super::*` glob (auth precedent) invites unused-import / clippy churn. | Low | Prefer explicit imports per submodule; clippy `-D warnings` gate catches leftovers. |
| R6 | File is an active **churn hotspot** — a big-bang split conflicts with any in-flight PR (#2838/#2834 landed recently). | High | **Stage it** (Section 5); land during a quiet window; rebase each stage on `dev` before pushing. Never bundle a move with a behavior change. |

---

## 5. Staged sequence

Each stage is independently shippable and must land green (`just verify` +
`cargo test -p api-server`). Dedup-in-place comes **first** (small, reviewable
diffs, no import churn); pure leaves move before the security-hot handlers.

| Stage | Scope | Why this order |
| --- | --- | --- |
| **0** | In-place: add `VoiceError`/`err_response` (W4) + `process_voice_command` (W3). No files moved. | Pure dedup, trivially reviewable, shrinks later move diffs. |
| **1** | In-place: collapse the two rate-limiter blocks (W1). | Self-contained; unit tests already cover it. |
| **2** | In-place: extract `mint_encrypted_tokens` OAuth ladder (W2). | Removes D2 before the handlers move, so Stage 5 moves less code. |
| **3** | Create `voice_webhooks/mod.rs` shell; move **pure leaves** — `response.rs`, `crypto.rs`, `auth.rs` — with their tests. | Lowest-risk (mostly pure fns); establishes the directory + `mod.rs` router. |
| **4** | Move signature verification into `signature/{mod,alexa,google}.rs` with its PEM-fixture tests. | Large but self-contained crypto; independent of the OAuth handlers. |
| **5** | Move `oauth.rs` + `handlers.rs` (the security-hot handlers) last. Router stays in `mod.rs`. | Done when shared helpers already exist and the remaining surface is smallest. |
| **6** | Cleanup: confirm utoipa/OpenAPI build, full `cargo test -p api-server`, `cargo sqlx prepare`, clippy `-D warnings`. | Final gate. |

Stages 0–2 deliver the concrete duplication wins **even if the directory split
(3–6) is deferred**, so value lands early and the risky moves are optional.

---

## 6. Non-goals / out of scope

- No behavior change and no endpoint-contract change — a pure structural refactor.
- No new dependency; no move to Redis-backed rate limiting (separate concern).
- Test coverage stays exhaustive — tests move, they are not dropped or thinned.
- `voice_assistant_devices` RLS posture (currently not RLS-bound; see migrations
  00226/00231) is unchanged by this plan.
