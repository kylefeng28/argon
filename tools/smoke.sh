#!/usr/bin/env bash
# Headless smoke test: run every demo for a few frames and screenshot the
# shell and editor. Exits non-zero if anything fails.
#   source tools/headless-env.sh && tools/smoke.sh [out-dir]
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${1:-target/smoke}
DISK=$(mktemp -d)
mkdir -p "$OUT"
cargo build -q
A=./target/debug/argon

for f in disk/DEMOS/*.lua disk/DEMOS/shadertoy/*.glsl; do
  rel=${f#disk/DEMOS/}; rel=${rel%.lua}; rel=${rel%.glsl}
  "$A" --disk "$DISK" check "DEMOS/$rel"
  # Give input-driven demos something to chew on, then screenshot.
  "$A" --disk "$DISK" run "DEMOS/$rel" --headless --frames 45 \
    --keys "{wait:10}abc{enter}{wait:10}{right}{hold:z}{wait:5}{release:z}" \
    --screenshot "$OUT/demo-${rel//\//-}.png" 2>/dev/null
done

"$A" --disk "$DISK" shell --headless --frames 30 --keys "help{enter}2^10{enter}dir{enter}" \
  --screenshot "$OUT/shell.png" --text 2>/dev/null | grep -q 1024
"$A" --disk "$DISK" shell --headless --frames 30 --keys "new smoke{enter}{wait:3}{f1}fill_ci" \
  --screenshot "$OUT/editor.png" 2>/dev/null
"$A" --disk "$DISK" shell --headless --frames 60 --keys "run demos/hello{enter}{wait:5}Kyle{enter}{wait:5}{space}{wait:5}" \
  --screenshot "$OUT/hello-flow.png" --text 2>/dev/null | grep -q "Nice to meet you, Kyle"

rm -rf "$DISK"
echo "smoke OK -> $OUT"
