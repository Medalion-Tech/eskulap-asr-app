#!/usr/bin/env bash
# Patches the bundled llama.cpp (vendored inside llama-cpp-sys-4) to work
# around TWO independent issues:
#
# Patch 1 — disable fused Gated Delta Net (FGDN) in llama-context.cpp
# ------------------------------------------------------------------
# Upstream hard-codes in src/llama-context.cpp:
#   cparams.fused_gdn_ar = true;  // autoregressive path
#   cparams.fused_gdn_ch = true;  // chunked path
# and then runs a verification block on first decode that aborts with:
#   GGML_ASSERT(strncmp(n->name, LLAMA_TENSOR_NAME_FGDN_AR "-", prefix_len) == 0)
#   GGML_ASSERT(strncmp(n->name, LLAMA_TENSOR_NAME_FGDN_CH "-", prefix_len) == 0)
# because the graph builder names the tensor "__fgdn_ar__" / "__fgdn_ch__"
# (no layer suffix) while the assertion expects "__fgdn_xx__-<il>".
# Setting both flags to false routes code through the pre-fusion fallback
# (build_delta_net_autoregressive / _chunked) — identical results, ~slower.
#
# Patch 2 — install llama-common-base alongside llama-common (CMakeLists.txt)
# --------------------------------------------------------------------------
# Since llama.cpp 0.2.46 split the legacy `llama-common` into two static
# libraries — `llama-common` (common.cpp, arg.cpp, chat.cpp, …) and
# `llama-common-base` (build-info.cpp, generated at configure time with
# llama_build_number / llama_commit / llama_compiler / llama_build_target).
# Upstream CMakeLists.txt only installs the first:
#   if (LLAMA_BUILD_COMMON)
#       install(TARGETS llama-common LIBRARY)
#   endif()
# which means cmake-rs's install-prefix-scanning glob picks up
# `llama-common.lib` but NOT `llama-common-base.lib`. Rust then packs
# common.cpp.obj into the crate's rlib, which references the 4 build-info
# symbols — and final `link.exe` errors out with LNK2019 unresolved
# externals on every Windows build (CPU / Vulkan / CUDA). Linux is
# unaffected because GNU ld lazily resolves static-lib symbols via its
# build-tree search paths. The fix is to add llama-common-base to the
# install line so cmake-rs sees it in the install prefix too.
#
# Both patches are idempotent: re-running the script no-ops on files
# already patched.

set -euo pipefail

REGISTRY_ROOT="${CARGO_HOME:-$HOME/.cargo}/registry/src"

if [ ! -d "$REGISTRY_ROOT" ]; then
  echo "Cargo registry not found at $REGISTRY_ROOT; run 'cargo fetch' first."
  exit 0
fi

# ── Patch 1 — FGDN workaround in llama-context.cpp ──────────────────────────
patch_fgdn() {
  local file=$1
  local ar_pat='cparams\.fused_gdn_ar = true;'
  local ch_pat='cparams\.fused_gdn_ch = true;'
  local ar_new='cparams.fused_gdn_ar = false; // PATCHED: Gemma 4 FGDN_AR workaround'
  local ch_new='cparams.fused_gdn_ch = false; // PATCHED: Gemma 4 FGDN_CH workaround'

  local changed=0
  if grep -q "$ar_pat" "$file" 2>/dev/null; then
    sed -i.bak "s|$ar_pat|$ar_new|" "$file"
    changed=1
  fi
  if grep -q "$ch_pat" "$file" 2>/dev/null; then
    sed -i.bak "s|$ch_pat|$ch_new|" "$file"
    changed=1
  fi
  rm -f "${file}.bak"

  if [ "$changed" -eq 1 ]; then
    echo "FGDN: patched $file"
    return 0
  fi
  if grep -q "// PATCHED: Gemma 4" "$file" 2>/dev/null; then
    echo "FGDN: already patched $file"
    return 0
  fi
  echo "FGDN: WARNING — patterns not found in $file (upstream API changed?)"
  return 0
}

# ── Patch 2 — install llama-common-base alongside llama-common ──────────────
patch_install() {
  local file=$1
  local marker='# PATCHED: install common-base too'
  if grep -qF "$marker" "$file" 2>/dev/null; then
    echo "install: already patched $file"
    return 0
  fi
  # Match the exact line and replace with a version that adds llama-common-base.
  # Pattern is narrow enough that we won't accidentally match any other install()
  # rule — upstream has exactly one `install(TARGETS llama-common LIBRARY)`.
  local old='install(TARGETS llama-common LIBRARY)'
  local new='install(TARGETS llama-common llama-common-base LIBRARY ARCHIVE) # PATCHED: install common-base too'
  if grep -qF "$old" "$file" 2>/dev/null; then
    # Escape for sed (parens and slashes are safe; use | as delim, escape parens for BRE).
    sed -i.bak "s|install(TARGETS llama-common LIBRARY)|$new|" "$file"
    rm -f "${file}.bak"
    echo "install: patched $file"
    return 0
  fi
  echo "install: WARNING — 'install(TARGETS llama-common LIBRARY)' not found in $file (upstream API changed?)"
  return 0
}

found_ctx=0
while IFS= read -r -d '' file; do
  found_ctx=$((found_ctx + 1))
  patch_fgdn "$file"
done < <(find "$REGISTRY_ROOT" -type f -path "*/llama-cpp-sys-4-*/llama.cpp/src/llama-context.cpp" -print0 2>/dev/null)

if [ "$found_ctx" -eq 0 ]; then
  echo "FGDN: no llama-cpp-sys-4 llama-context.cpp found. Run 'cargo fetch' in src-tauri first."
fi

found_cmk=0
while IFS= read -r -d '' file; do
  found_cmk=$((found_cmk + 1))
  patch_install "$file"
done < <(find "$REGISTRY_ROOT" -type f -path "*/llama-cpp-sys-4-*/llama.cpp/CMakeLists.txt" -not -path "*/llama.cpp/*/CMakeLists.txt" -print0 2>/dev/null)

if [ "$found_cmk" -eq 0 ]; then
  echo "install: no llama-cpp-sys-4 top-level CMakeLists.txt found. Run 'cargo fetch' in src-tauri first."
fi
