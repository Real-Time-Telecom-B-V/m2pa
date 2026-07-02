#!/usr/bin/env bash
#
# m2pa CPU flamegraph.
#
# Profiles the codec throughput driver (`perf_codec`) and writes `flamegraph.svg`,
# showing where encode/decode time goes (header pack/unpack, the body copy).
#
# Requires: `perf` + `cargo install flamegraph`, and perf sampling access:
#   sudo sysctl kernel.perf_event_paranoid=1     # (or run this script with sudo)
#
# Usage: ITERS=50000000 ./scripts/flamegraph.sh

set -euo pipefail
cd "$(dirname "$0")/.."

paranoid="$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo 99)"
if [ "$paranoid" -gt 1 ]; then
    echo "WARN: kernel.perf_event_paranoid=$paranoid — perf sampling needs <= 1."
    echo "      run:  sudo sysctl kernel.perf_event_paranoid=1   (then re-run)"
fi

export ITERS="${ITERS:-50000000}"

# Force frame pointers so perf can unwind the optimized stacks (otherwise every
# frame is [unknown]). This rebuilds deps once.
export RUSTFLAGS="-C force-frame-pointers=yes ${RUSTFLAGS:-}"

echo "[*] flamegraph: $ITERS iterations of the codec driver"
cargo flamegraph --freq 499 --profile profiling --example perf_codec --output flamegraph.svg
echo "[+] wrote flamegraph.svg"
