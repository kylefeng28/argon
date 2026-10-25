#!/usr/bin/env bash
# Source this to build and run Argon headless on a machine with no GPU driver
# and no ALSA development package (e.g. a bare Linux devbox).
#   source tools/headless-env.sh && cargo test
#
# 1. GPU: use the SwiftShader Vulkan ICD that ships with Playwright's Chromium.
CH=$(ls -d "$HOME"/.cache/ms-playwright/chromium-*/chrome-linux64 2>/dev/null | sort | head -1)
if [ -n "$CH" ] && [ -f "$CH/libvulkan.so.1" ]; then
  export LD_LIBRARY_PATH="$CH${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  export VK_ICD_FILENAMES="$CH/vk_swiftshader_icd.json"
  export WGPU_BACKEND=vulkan
  echo "argon: using SwiftShader from $CH"
else
  echo "argon: no SwiftShader found; relying on system GPU drivers" >&2
fi

# 2. Audio: cpal links against libasound through pkg-config. If only the
#    runtime library exists (no alsa-lib-devel), fake the .pc file.
if ! pkg-config --exists alsa 2>/dev/null && [ -f /usr/lib64/libasound.so.2 ]; then
  mkdir -p /tmp/alsa
  ln -sf /usr/lib64/libasound.so.2 /tmp/alsa/libasound.so
  cat > /tmp/alsa/alsa.pc <<'EOF'
prefix=/tmp/alsa
libdir=${prefix}
Name: alsa
Description: runtime-only alsa for linking
Version: 1.1.0
Libs: -L${libdir} -lasound
Cflags:
EOF
  export PKG_CONFIG_PATH="/tmp/alsa${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
  export LD_LIBRARY_PATH="/tmp/alsa${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  echo "argon: using runtime libasound via /tmp/alsa/alsa.pc"
fi
