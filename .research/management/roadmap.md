# PPT Roadmap (upkeep refresh)

Generated: 2026-06-17T00:00:00Z · scan_kind: upkeep · today's role: pm-security

## State of the project

Stories: done=29 partial=20 not-started=0 (total 49)

Per-platform breakdown:
- `backend`: done=21 partial=7 not-started=0
- `frontend`: done=15 partial=11 not-started=0
- `mobile`: done=4 partial=9 not-started=0

Screen coverage: 0 orphan epics · 0 orphan screens · 0 missing UC links

## Top gaps (from this run's PM rotation)

- **[high]** Fix CI #1538: backend test job not required on dev — undermines RLS/OAuth regression harness — owner: pm-tech-lead / pm-security
- **[high]** Fix #481 OAuth refresh-token revocation bypass (RFC 9700) — owner: pm-backend
- **[high]** Fix #480 JWT in WS access logs — credential exfiltration via log aggregators — owner: pm-backend
- **[high]** Land Epic 6 announcement web UI (draft PRs #474/#475/#479) — owner: pm-frontend
- **[medium]** Map native accounting MVP (#1454, 17,983 LOC) into coverage.json — owner: pm-backend

## Coverage upkeep this run

- Re-ranked epic-9 (TOTP 2FA): refreshed evidence with PR #1467 (BIT-78 MFA verify_recovery_code IDOR test); story 9-1 remains `done`.
- Stamped 8a-3 (notification preference sync) with PR #1450 evidence; remains `partial` (no re-classification this run).
- coverage_cursor advances 12 -> 0 (next rotating epic: epic-10a).

## Coverage status

**Buffer-low advisory from dispatcher:** claimable=22/72 below floor=36; coverage.json was stale (last full scan 2026-06-16) and exhausted (all current gaps already in backlog/done). Recommend running on-demand local `/ppt-project-management scan` to refill candidate pool.

Buffer: 50/36 open · 0 candidates ranked but unqueued (upkeep mode does not re-rank candidates without scan).
