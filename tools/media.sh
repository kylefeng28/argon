#!/usr/bin/env bash
# Record README media: a PNG still and a looping GIF per demo, plus shell and
# editor stills. Uses a throwaway HOME so your own config does not leak in.
#   tools/media.sh [out-dir]          (needs ffmpeg)
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${1:-docs/media}
THEME=${ARGON_MEDIA_THEME:-grid}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$OUT" "$TMP/home/.argon"
echo $TMP
# printf 'theme = "%s"\n' "$THEME" > "$TMP/home/.argon/config.toml"
cat > "$TMP/home/.argon/config.toml" <<EOF
theme = "$THEME"
filter = "none"
EOF
cargo build -q --release
A="$PWD/target/release/argon"
argon() { HOME="$TMP/home" "$A" --disk "$TMP/disk" "$@"; }

# name  warm-up-frames  kind (gif or png)  [keys]
DEMOS=(
  "particles_gpu 60 gif"
  "reaction_diffusion 240 gif"
  "julia_gpu 30 gif"
  "raymarch 30 gif {mouse:900,500}"
  "attractor 120 gif"
  "hud 30 gif"
  "matrix 60 gif"
  "chiptune 30 gif"
  "cube 30 gif"
  "mandelbrot_gpu 30 png"
  "shaderfx 30 png"
  "flock 30 gif"
  "life 30 gif"
)
SECS=3

for d in "${DEMOS[@]}"; do
  read -r name from kind keys <<<"$d"
  frames=$from
  rec=()
  if [ "$kind" = gif ]; then
    frames=$((from + SECS * 60))
    rm -rf "$TMP/frames"
    rec=(--record "$TMP/frames" --record-every 4 --record-from "$from")
  fi
  argon run "DEMOS/$name" --headless --frames "$frames" ${keys:+--keys "$keys"} \
    ${rec[@]+"${rec[@]}"} --screenshot "$OUT/$name.png" 2>/dev/null
  if [ "$kind" = gif ]; then
    # 15 fps, 480 wide, one shared palette per clip, to keep each GIF small.
    ffmpeg -loglevel error -y -framerate 15 -i "$TMP/frames/frame-%05d.png" \
      -vf "scale=480:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle" \
      -loop 0 "$OUT/$name.gif"
  fi
  echo "$name"
done

argon shell --headless --frames 30 --keys "help{enter}" --screenshot "$OUT/shell.png" 2>/dev/null
argon shell --headless --frames 40 --keys "edit demos/bounce{enter}{wait:5}{f1}" \
  --screenshot "$OUT/editor.png" 2>/dev/null
du -sh "$OUT"
