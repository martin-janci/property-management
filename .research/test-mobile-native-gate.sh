#!/usr/bin/env bash
# Smoke test for mobile-native-gate.sh — deterministic, offline (fixture-fed).
# Covers: id-token signal (positive + boundary negatives), plan Files-scope
# signal (pure-KMP positive, cross-stack negative, test-only-overlap allowed),
# single-mode exit codes, and batch mode over an action-list fixture.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/mobile-native-gate.sh"
FAIL=0
ok()  { printf '  ok    %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1"; FAIL=1; }

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
PLANS="$TMP/plans"; mkdir -p "$PLANS"
AL="$TMP/action-list.json"

# gated? single-mode helper: prints "gated" / "notgated" based on exit code.
g() { if bash "$SCRIPT" --id "$1" --plans-dir "$PLANS" >/dev/null 2>&1; then echo gated; else echo notgated; fi; }

echo "== mobile-native-gate smoke test =="

# --- 1. id-token signal: positives -----------------------------------------
for id in code-review-mobile-native-kmp-android-sso-deeplink \
          churn-hotspot-mobile-native-shared-networkclient \
          gap-42-kmp-shared-auth \
          pm-mobile-native-refactor; do
  [ "$(g "$id")" = gated ] && ok "id '$id' gated (id token)" || bad "id '$id' NOT gated"
done

# --- 2. id-token signal: boundary NEGATIVES --------------------------------
# mobile-rn (React Native) is verifiable in cloud — must NOT be gated.
# frontend/apps/mobile is React Native too. Substrings inside a word must not trip.
for id in code-review-mobile-rn-report-fault-fake-submit \
          gap-7-frontend-apps-mobile-fault-list \
          churn-hotspot-backend-servers-api-server \
          some-kmpish-word-thing; do
  [ "$(g "$id")" = notgated ] && ok "id '$id' NOT gated (boundary)" || bad "id '$id' wrongly gated"
done

# --- 3. plan Files-scope signal --------------------------------------------
# 3a. pure mobile-native plan (no id token) -> gated by files
cat > "$PLANS/plan-pure-kmp.md" <<'MD'
# plan-pure-kmp
## Files
- `mobile-native/shared/src/commonMain/kotlin/Foo.kt`
- `mobile-native/androidApp/src/main/kotlin/Bar.kt:42`
## Dependencies
MD
[ "$(g plan-pure-kmp)" = gated ] && ok "pure-KMP plan gated (files scope)" || bad "pure-KMP plan NOT gated"

# 3b. cross-stack plan (backend + mobile-native) -> NOT gated (has verifiable slice)
cat > "$PLANS/plan-cross-stack.md" <<'MD'
# plan-cross-stack
## Files
- `backend/crates/api-core/src/lib.rs`
- `mobile-native/shared/src/commonMain/kotlin/Foo.kt`
## Dependencies
MD
[ "$(g plan-cross-stack)" = notgated ] && ok "cross-stack plan NOT gated" || bad "cross-stack plan wrongly gated"

# 3c. mobile-native + test-only other path -> gated (test paths don't disqualify)
cat > "$PLANS/plan-kmp-plus-test.md" <<'MD'
# plan-kmp-plus-test
## Files
- `mobile-native/shared/src/commonMain/kotlin/Foo.kt`
- `mobile-native/shared/src/commonTest/kotlin/FooTest.kt`
- `backend/crates/api-core/tests/it.rs`
## Dependencies
MD
[ "$(g plan-kmp-plus-test)" = gated ] && ok "KMP + test-only overlap gated" || bad "KMP + test-only NOT gated"

# 3d. no plan, no id signal -> not gated
[ "$(g totally-unrelated-backend-task)" = notgated ] && ok "no plan + no id signal NOT gated" || bad "unrelated task wrongly gated"

# --- 4. single-mode exit codes + output ------------------------------------
if bash "$SCRIPT" --id code-review-mobile-native-foo --plans-dir "$PLANS" >/dev/null 2>&1; then
  ok "single mode exit 0 for gated id"
else bad "single mode did not exit 0 for gated id"; fi
OUT=$(bash "$SCRIPT" --id code-review-mobile-native-foo --plans-dir "$PLANS" 2>/dev/null || true)
echo "$OUT" | grep -q 'gated=true signal=id' && ok "single-mode line: gated=true signal=id" || bad "single-mode line wrong: $OUT"
if bash "$SCRIPT" --id plain-backend-task --plans-dir "$PLANS" >/dev/null 2>&1; then
  bad "single mode exit 0 for NON-gated id"
else ok "single mode exit 1 for non-gated id"; fi
bash "$SCRIPT" --bogus >/dev/null 2>&1 && bad "usage error did not exit nonzero" || ok "usage error exits nonzero"

# --- 5. batch mode over an action-list -------------------------------------
cat > "$AL" <<'JSON'
{"generated":"2026-08-04T00:00:00Z","items":[
  {"id":"code-review-mobile-native-kmp-sso","status":"open"},
  {"id":"churn-hotspot-mobile-native-shared","status":"open"},
  {"id":"code-review-mobile-rn-fault","status":"open"},
  {"id":"gap-9-backend-auth","status":"open"},
  {"id":"code-review-mobile-native-already-done","status":"done"},
  {"id":"plan-pure-kmp","status":"open"}
]}
JSON
cp "$PLANS/plan-pure-kmp.md" "$PLANS/plan-pure-kmp.md" 2>/dev/null || true
BATCH=$(bash "$SCRIPT" --action-list "$AL" --plans-dir "$PLANS" 2>/dev/null | cut -f1 | sort | tr '\n' ',')
EXP="churn-hotspot-mobile-native-shared,code-review-mobile-native-kmp-sso,plan-pure-kmp,"
[ "$BATCH" = "$EXP" ] && ok "batch gated set correct (open only, rn/backend excluded)" || bad "batch set=[$BATCH] expected=[$EXP]"

echo
if [ "$FAIL" = "0" ]; then echo "==> mobile-native-gate smoke test PASSED"; exit 0
else echo "==> mobile-native-gate smoke test FAILED"; exit 1; fi
