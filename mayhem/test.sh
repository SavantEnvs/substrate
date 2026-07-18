#!/usr/bin/env bash
#
# mayhem/test.sh — RUN the sp-arithmetic behavioral oracle integration tests
# (pre-compiled by mayhem/build.sh in the fuzz workspace).
#
# Anti-reward-hack: asserts on specific MATHEMATICAL results (known-answer tests),
# not just exit status. A PATCH that neuters sp-arithmetic to exit(0) changes
# arithmetic results and fails multiple assertions. Uses the fuzz workspace's
# nightly-2025-05-14-compatible deps (hashbrown 0.17.x with bundled ahash).
# Emits CTRF. Does NOT compile (build.sh did).
set -uo pipefail
[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH
: "${MAYHEM_JOBS:=$(nproc)}"
cd "$SRC"

emit_ctrf() {
  local tool="$1" passed="$2" failed="$3" skipped="${4:-0}" pending="${5:-0}" other="${6:-0}"
  local tests=$(( passed + failed + skipped + pending + other ))
  cat > "${CTRF_REPORT:-$SRC/ctrf-report.json}" <<JSON
{
  "results": {
    "tool": { "name": "$tool" },
    "summary": {
      "tests": $tests,
      "passed": $passed,
      "failed": $failed,
      "pending": $pending,
      "skipped": $skipped,
      "other": $other
    }
  }
}
JSON
  printf 'CTRF {"results":{"tool":{"name":"%s"},"summary":{"tests":%d,"passed":%d,"failed":%d,"pending":%d,"skipped":%d,"other":%d}}}\n' \
    "$tool" "$tests" "$passed" "$failed" "$pending" "$skipped" "$other"
  [ "$failed" -eq 0 ]
}

# Locate the prebuilt integration test binary from the fuzz workspace target dir.
# cargo names integration tests as <test-name>-<hash> in target/debug/deps/.
TEST_TARGET_DIR="$SRC/mayhem/fuzz/target"

mapfile -t RUNNERS < <(find "$TEST_TARGET_DIR/debug/deps" -maxdepth 1 \
    -type f -name 'sp_arithmetic_tests-*' -executable 2>/dev/null)

if [ "${#RUNNERS[@]}" -eq 0 ]; then
  echo "FATAL: no prebuilt test runner under $TEST_TARGET_DIR/debug/deps" >&2
  echo "       build.sh should have produced sp_arithmetic_tests-* via cargo test --no-run" >&2
  emit_ctrf "cargo-test" 0 1
  exit 1
fi

passed_total=0
failed_total=0
saw_marker=0

for runner in "${RUNNERS[@]}"; do
  echo "=== running $runner ==="
  out="$("$runner" --test-threads="$MAYHEM_JOBS" 2>&1)" && rc=0 || rc=$?
  echo "$out"
  while IFS= read -r line; do
    if [[ "$line" =~ test\ result:.*\ ([0-9]+)\ passed\;\ ([0-9]+)\ failed ]]; then
      passed_total=$(( passed_total + ${BASH_REMATCH[1]} ))
      failed_total=$(( failed_total + ${BASH_REMATCH[2]} ))
      saw_marker=1
    fi
  done <<< "$out"
  if [ "$rc" -ne 0 ] && [ "$saw_marker" -eq 0 ]; then
    failed_total=$(( failed_total + 1 ))
  fi
done

if [ "$saw_marker" -eq 0 ]; then
  echo "FATAL: no libtest 'test result:' marker seen — suite did not run" >&2
  emit_ctrf "cargo-test" 0 1
  exit 1
fi

# The integration test suite has 16 known-answer tests; require at least 15 to pass.
# A neutered sp-arithmetic produces wrong arithmetic results -> fails the assertions.
if [ "$passed_total" -lt 15 ]; then
  echo "FATAL: only $passed_total tests passed — expected at least 15 from sp-arithmetic" >&2
  emit_ctrf "cargo-test" "$passed_total" $(( failed_total > 0 ? failed_total : 1 ))
  exit 1
fi

emit_ctrf "cargo-test" "$passed_total" "$failed_total"
