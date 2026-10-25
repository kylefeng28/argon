#!/usr/bin/env bash
# Source this to run Argon headless on a machine with no GPU driver, using the
# SwiftShader Vulkan ICD that ships with Playwright's Chromium.
#   source tools/headless-env.sh && cargo test
CH=$(ls -d "$HOME"/.cache/ms-playwright/chromium-*/chrome-linux64 2>/dev/null | sort | head -1)
if [ -n "$CH" ] && [ -f "$CH/libvulkan.so.1" ]; then
  export LD_LIBRARY_PATH="$CH${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  export VK_ICD_FILENAMES="$CH/vk_swiftshader_icd.json"
  export WGPU_BACKEND=vulkan
  echo "argon: using SwiftShader from $CH"
else
  echo "argon: no SwiftShader found; relying on system GPU drivers" >&2
fi
