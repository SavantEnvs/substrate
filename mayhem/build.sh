#!/usr/bin/env bash
#
# mayhem/build.sh — build substrate's libFuzzer fuzz target (sp-arithmetic) as a
# sanitized binary, then pre-compile the behavioral oracle integration tests so
# mayhem/test.sh only needs to RUN them.
#
# Runs inside the commit image (RUST mayhem/Dockerfile) as `mayhem` in /mayhem.
# Toolchain lives at $CARGO_HOME=/opt/toolchains/rust/cargo (pinned by Dockerfile ENV).
#
# AIR-GAPPED CONTRACT (SPEC §6.5): the PATCH tier re-runs THIS script OFFLINE.
#   - First (online) build: populates $CARGO_HOME; Dockerfile generated a fresh
#     Cargo.lock for mayhem/fuzz/ with compatible nightly-2025-05-14 deps
#     (hashbrown 0.17.x bundles ahash internally — no legacy ahash 0.7.6 / 0.8.3).
#   - PATCH re-run: resolves from cache (CARGO_NET_OFFLINE=true exported by runtime).
#   - Do NOT hard-code --offline (breaks the first online build).
set -euo pipefail

[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH

: "${MAYHEM_JOBS:=$(nproc)}"
export CARGO_BUILD_JOBS="$MAYHEM_JOBS"

cd "$SRC"

# Sanitizers: non-empty SANITIZER_FLAGS -> ASan; empty -> un-sanitized build.
RUST_SAN=""
if [ -n "${SANITIZER_FLAGS:-}" ]; then
  RUST_SAN="-Zsanitizer=address"
fi

# DWARF < 4 gate: pin Rust to DWARF-3; pin cc-compiled shim too.
export RUSTFLAGS="${RUSTFLAGS:-} ${RUST_DEBUG_FLAGS:-} --cfg fuzzing ${RUST_SAN} -Zdwarf-version=3 -Cdebuginfo=1 -Cforce-frame-pointers"
export CFLAGS="${CFLAGS:-} -gdwarf-3"
export CXXFLAGS="${CXXFLAGS:-} -gdwarf-3"

# Strip DWARF-5 from the bundled ASan runtime archive. Idempotent.
if [ -n "${RUST_SAN}" ]; then
  RT_LIB_DIR="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/lib"
  for asan in "$RT_LIB_DIR"/librustc-*_rt.asan.a; do
    [ -f "$asan" ] || continue
    if [ -w "$asan" ]; then
      objcopy --strip-debug "$asan" "$asan.stripped" && mv "$asan.stripped" "$asan"
      echo "stripped debug info from bundled ASan runtime: $asan"
    fi
  done
fi

FUZZ_DIR="mayhem/fuzz"
TRIPLE="x86_64-unknown-linux-gnu"

# Discover fuzz targets.
FUZZ_TARGETS=()
for f in "$FUZZ_DIR"/fuzz_targets/*.rs; do
  FUZZ_TARGETS+=("$(basename "${f%.*}")")
done
[ "${#FUZZ_TARGETS[@]}" -gt 0 ] || { echo "ERROR: no fuzz targets under $FUZZ_DIR/fuzz_targets/" >&2; exit 1; }

echo "=== cargo fuzz build (image nightly, ASan via RUSTFLAGS) ==="
echo "RUSTFLAGS=$RUSTFLAGS"
echo "targets: ${FUZZ_TARGETS[*]}"

for t in "${FUZZ_TARGETS[@]}"; do
  echo "--- building fuzz target: $t ---"
  cargo fuzz build --fuzz-dir "$FUZZ_DIR" -O --debug-assertions "$t"
  bin="$SRC/$FUZZ_DIR/target/$TRIPLE/release/$t"
  [ -x "$bin" ] || { echo "ERROR: expected fuzz binary not found at $bin" >&2; exit 1; }
  cp "$bin" "/mayhem/$t"
  echo "built /mayhem/$t"
done

# Build the behavioral oracle integration tests (in the FUZZ workspace).
# The fuzz workspace has a fresh Cargo.lock with nightly-compatible deps
# (hashbrown 0.17.x bundles ahash — no legacy ahash 0.7.6 / 0.8.3 issue).
# Debug build (not Release) so assert!() macros remain active.
echo "=== building sp-arithmetic integration tests (cargo test --no-run) ==="
env -u RUSTFLAGS -u CFLAGS -u CXXFLAGS \
  cargo test --no-run --manifest-path "$FUZZ_DIR/Cargo.toml" --test sp_arithmetic_tests 2>&1
echo "build.sh complete"
