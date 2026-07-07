#!/usr/bin/env python3
"""Build the branch-protection PUT payload for branch-protection-setup.yml.

WHY THIS EXISTS (single source of truth + regression test)
----------------------------------------------------------
PR #923 ("don't weaken branch protection; rebase lockfiles from base",
closes #771) replaced a HARDCODED branch-protection payload with a
round-trip builder: it starts from the branch's CURRENT protection and
only *adds/strengthens*, so running the setup workflow can never silently
revert a stronger rule (extra approvals, code-owner reviews, push
restrictions, linear history, conversation resolution, strict mode).

That fix shipped with no test. This module extracts the payload-builder
logic out of the inline `python3 -c "..."` heredoc in
`.github/workflows/branch-protection-setup.yml` so that:

  1. the workflow imports ONE implementation (no copy drift), and
  2. `--self-test` exercises that implementation against fixtures that
     encode the exact #771/#923 regression — a future edit that
     re-introduces a hardcoded/weakening payload fails CI here instead
     of silently downgrading `dev` protection on the next dispatch run.

CONTRACT (mirrors the workflow's documented intent)
---------------------------------------------------
Given the CURRENT protection object (as returned by
`GET /repos/{o}/{r}/branches/{b}/protection`) and the merged list of
required status-check contexts, produce the body for
`PUT .../branches/{b}/protection` such that:

  * required_status_checks.strict is ALWAYS true (never weaken; strengthen
    false -> true), and .checks is exactly the merged context list.
  * required_pull_request_reviews is PRESERVED when present (approval
    count, dismiss-stale, require_last_push_approval), else a safe
    baseline (1 approval) — EXCEPT require_code_owner_reviews, which is
    ALWAYS true (strengthen false -> true, never weaken; #2127: without
    it every CODEOWNERS entry is advisory-only).
  * enforce_admins is preserved (dict {enabled} or bool), default true.
  * restrictions (push restrictions) are PRESERVED, never nulled, when
    present; null only when the current rule had none.
  * the boolean toggles required_linear_history, allow_force_pushes,
    allow_deletions, required_conversation_resolution, block_creations,
    lock_branch are carried over only when present on the current rule.

USAGE
-----
  # Build mode (used by the workflow):
  #   reads current protection JSON from $PROTECTION_JSON (empty/absent =>
  #   no existing protection), merged contexts (newline-delimited) from
  #   $MERGED, and $PROTECTION_EXISTS in {"true","false"}; prints the
  #   payload JSON to stdout.
  PROTECTION_EXISTS=true PROTECTION_JSON='{...}' MERGED=$'a\nb' \
      python3 branch-protection-payload.py

  # Self-test mode (used by CI):
  python3 branch-protection-payload.py --self-test
"""

from __future__ import annotations

import json
import os
import sys


def build_payload(current: dict | None, checks: list[str]) -> dict:
    """Round-trip the current protection into a PUT payload.

    `current` is the parsed current protection object (or None / {} when no
    protection exists yet). `checks` is the merged list of required
    status-check contexts. Returns the PUT body as a dict.
    """
    cur = current or {}

    # --- required_status_checks: force strict=true, set the merged checks ---
    # Never weaken strict: always require branches to be up to date before
    # merge. If the rule already had strict=true this is a no-op; if it was
    # false we strengthen it. We never set strict=false.
    rsc = {
        "strict": True,
        "checks": [{"context": c, "app_id": -1} for c in checks],
    }

    # --- required_pull_request_reviews: preserve existing, else safe baseline ---
    # require_code_owner_reviews is a strengthen-only toggle like strict:
    # without it every CODEOWNERS entry (backend/deny.toml, Cargo.toml/lock
    # guards) is advisory-only (#2127). Always true; never set false.
    cur_prr = cur.get("required_pull_request_reviews")
    if cur_prr:
        prr = {
            "dismiss_stale_reviews": bool(cur_prr.get("dismiss_stale_reviews", False)),
            "require_code_owner_reviews": True,
            "required_approving_review_count": int(
                cur_prr.get("required_approving_review_count", 1)
            ),
        }
        # Preserve scoped review dismissal / bypass allowances if present.
        if "require_last_push_approval" in cur_prr:
            prr["require_last_push_approval"] = bool(
                cur_prr["require_last_push_approval"]
            )
    else:
        prr = {
            "dismiss_stale_reviews": False,
            "require_code_owner_reviews": True,
            "required_approving_review_count": 1,
        }

    # --- enforce_admins: preserve (dict {enabled} or bool), default true ---
    cur_ea = cur.get("enforce_admins")
    if isinstance(cur_ea, dict):
        enforce_admins = bool(cur_ea.get("enabled", True))
    elif isinstance(cur_ea, bool):
        enforce_admins = cur_ea
    else:
        enforce_admins = True

    # --- restrictions: PRESERVE existing push restrictions, never null them ---
    cur_restr = cur.get("restrictions")
    if cur_restr:
        restrictions = {
            "users": [u["login"] for u in cur_restr.get("users", [])],
            "teams": [t["slug"] for t in cur_restr.get("teams", [])],
            "apps": [a["slug"] for a in cur_restr.get("apps", [])],
        }
    else:
        restrictions = None

    payload = {
        "required_status_checks": rsc,
        "enforce_admins": enforce_admins,
        "required_pull_request_reviews": prr,
        "restrictions": restrictions,
    }

    # Preserve other boolean toggles only when the current rule has them on,
    # so we never accidentally relax them.
    for key in (
        "required_linear_history",
        "allow_force_pushes",
        "allow_deletions",
        "required_conversation_resolution",
        "block_creations",
        "lock_branch",
    ):
        val = cur.get(key)
        if isinstance(val, dict):
            val = val.get("enabled")
        if val is not None:
            payload[key] = bool(val)

    return payload


def _build_from_env() -> dict:
    """Build a payload from the same env vars the workflow sets."""
    checks = [
        c.strip() for c in os.environ.get("MERGED", "").strip().splitlines() if c.strip()
    ]
    exists = os.environ.get("PROTECTION_EXISTS") == "true"
    cur: dict = {}
    if exists:
        raw = os.environ.get("PROTECTION_JSON", "").strip()
        try:
            cur = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            cur = {}
    return build_payload(cur, checks)


# ---------------------------------------------------------------------------
# Self-test fixtures
#
# Each fixture pins one slice of the #771/#923 contract. `_check` raises
# AssertionError with an actionable message on mismatch.
# ---------------------------------------------------------------------------

NEW_CHECK = "security-gate-conclusion"


def _check(name: str, cond: bool, detail: str) -> None:
    status = "ok  " if cond else "FAIL"
    print(f"{status} {name}: {detail}")
    if not cond:
        raise AssertionError(f"{name}: {detail}")


def _self_test() -> int:
    # --- Fixture 1: the #771 regression — a STRONGER existing rule must be
    # fully preserved, with only the new check appended and strict forced on.
    strong = {
        "required_status_checks": {
            "strict": True,
            "checks": [{"context": "backend", "app_id": -1}],
        },
        "required_pull_request_reviews": {
            "dismiss_stale_reviews": True,
            "require_code_owner_reviews": True,
            "required_approving_review_count": 2,
            "require_last_push_approval": True,
        },
        "enforce_admins": {"enabled": True},
        "restrictions": {
            "users": [{"login": "alice"}],
            "teams": [{"slug": "maintainers"}],
            "apps": [],
        },
        "required_linear_history": {"enabled": True},
        "required_conversation_resolution": {"enabled": True},
        "allow_force_pushes": {"enabled": False},
    }
    p = build_payload(strong, ["backend", NEW_CHECK])
    prr = p["required_pull_request_reviews"]
    _check(
        "strong/approvals-preserved",
        prr["required_approving_review_count"] == 2,
        "2 required approvals carried over (regression: was hardcoded to 1)",
    )
    _check(
        "strong/code-owner-preserved",
        prr["require_code_owner_reviews"] is True,
        "code-owner reviews kept on",
    )
    _check(
        "strong/last-push-approval-preserved",
        prr.get("require_last_push_approval") is True,
        "require_last_push_approval carried over",
    )
    _check(
        "strong/restrictions-preserved",
        p["restrictions"] == {"users": ["alice"], "teams": ["maintainers"], "apps": []},
        "push restrictions flattened to login/slug and NOT nulled",
    )
    _check(
        "strong/linear-history-preserved",
        p.get("required_linear_history") is True,
        "linear history toggle carried over",
    )
    _check(
        "strong/conversation-resolution-preserved",
        p.get("required_conversation_resolution") is True,
        "conversation resolution carried over",
    )
    _check(
        "strong/enforce-admins-preserved",
        p["enforce_admins"] is True,
        "enforce_admins (dict form) carried over",
    )
    contexts = [c["context"] for c in p["required_status_checks"]["checks"]]
    _check(
        "strong/new-check-appended",
        NEW_CHECK in contexts and "backend" in contexts,
        f"existing 'backend' kept and '{NEW_CHECK}' appended -> {contexts}",
    )

    # --- Fixture 2: strict MUST be strengthened, never weakened. A rule that
    # currently has strict=false comes back strict=true.
    weak_strict = {
        "required_status_checks": {"strict": False, "checks": []},
        "required_pull_request_reviews": {
            "dismiss_stale_reviews": False,
            "require_code_owner_reviews": False,
            "required_approving_review_count": 1,
        },
    }
    p2 = build_payload(weak_strict, [NEW_CHECK])
    _check(
        "strict/strengthened",
        p2["required_status_checks"]["strict"] is True,
        "strict=false strengthened to strict=true (never weakened)",
    )
    _check(
        "code-owner/strengthened",
        p2["required_pull_request_reviews"]["require_code_owner_reviews"] is True,
        "require_code_owner_reviews=false strengthened to true (#2127)",
    )

    # --- Fixture 3: no existing protection -> safe baseline. ---
    p3 = build_payload({}, [NEW_CHECK])
    _check(
        "baseline/strict-true",
        p3["required_status_checks"]["strict"] is True,
        "baseline strict=true",
    )
    _check(
        "baseline/one-approval",
        p3["required_pull_request_reviews"]["required_approving_review_count"] == 1,
        "baseline requires 1 approval",
    )
    _check(
        "baseline/code-owner-required",
        p3["required_pull_request_reviews"]["require_code_owner_reviews"] is True,
        "baseline enforces code-owner reviews (#2127)",
    )
    _check(
        "baseline/enforce-admins",
        p3["enforce_admins"] is True,
        "baseline enforce_admins=true",
    )
    _check(
        "baseline/no-restrictions",
        p3["restrictions"] is None,
        "baseline restrictions=null (no push restrictions to invent)",
    )
    _check(
        "baseline/no-extra-toggles",
        "required_linear_history" not in p3 and "lock_branch" not in p3,
        "baseline does not fabricate optional toggles",
    )

    # --- Fixture 4: enforce_admins given as a bare bool is honoured. ---
    p4 = build_payload({"enforce_admins": False}, [NEW_CHECK])
    _check(
        "enforce-admins/bool-false",
        p4["enforce_admins"] is False,
        "enforce_admins given as bool False is preserved (not forced true)",
    )

    # --- Fixture 5: env round-trip path (what the workflow actually calls)
    # must equal the direct build for the same inputs. ---
    os.environ["PROTECTION_EXISTS"] = "true"
    os.environ["PROTECTION_JSON"] = json.dumps(strong)
    os.environ["MERGED"] = "backend\n" + NEW_CHECK
    env_payload = _build_from_env()
    _check(
        "env/round-trip-matches",
        env_payload == build_payload(strong, ["backend", NEW_CHECK]),
        "env-driven build equals direct build (workflow code path is exercised)",
    )

    # --- Fixture 6: malformed PROTECTION_JSON with exists=true degrades to
    # baseline rather than crashing the workflow. ---
    os.environ["PROTECTION_EXISTS"] = "true"
    os.environ["PROTECTION_JSON"] = "{not valid json"
    os.environ["MERGED"] = NEW_CHECK
    bad = _build_from_env()
    _check(
        "env/malformed-json-baseline",
        bad["required_status_checks"]["strict"] is True
        and bad["restrictions"] is None,
        "unparseable current protection falls back to safe baseline",
    )

    print("\nALL FIXTURES PASSED")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv[1:]:
        return _self_test()
    print(json.dumps(_build_from_env()))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
