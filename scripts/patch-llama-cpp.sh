#!/usr/bin/env bash
# Disables fused Gated Delta Net in the bundled llama.cpp to avoid FGDN
# assertion crashes that hit when running Gemma 4 E4B.
#
# Upstream llama.cpp (and the version pinned by llama-cpp-sys-4 0.2.x)
# hard-codes in src/llama-context.cpp:
#   cparams.fused_gdn_ar = true;  // autoregressive path
#   cparams.fused_gdn_ch = true;  // chunked path
# and then runs a verification block on first decode that aborts with:
#   GGML_ASSERT(strncmp(n->name, LLAMA_TENSOR_NAME_FGDN_AR "-", prefix_len) == 0)
#   GGML_ASSERT(strncmp(n->name, LLAMA_TENSOR_NAME_FGDN_CH "-", prefix_len) == 0)
# because the graph builder names the tensor "__fgdn_ar__" / "__fgdn_ch__"
# (no layer suffix) while the assertion expects "__fgdn_xx__-<il>".
#
# Setting both flags to false routes the code through the pre-fusion fallback
# (build_delta_net_autoregressive / _chunked) — identical results, slightly
# slower.
#
# Idempotent: re-running the script no-ops when the file is already patched.

set -euo pipefail

REGISTRY_ROOT="${CARGO_HOME:-$HOME/.cargo}/registry/src"

if [ ! -d "$REGISTRY_ROOT" ]; then
  echo "Cargo registry not found at $REGISTRY_ROOT; run 'cargo fetch' first."
  exit 0
fi

apply_patch() {
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
    echo "Patched: $file"
    return 0
  fi
  if grep -q "// PATCHED: Gemma 4" "$file" 2>/dev/null; then
    echo "Already patched: $file"
    return 1
  fi
  echo "WARNING: FGDN patterns not found in $file (upstream API changed?)"
  return 2
}

found=0
while IFS= read -r -d '' file; do
  found=$((found + 1))
  apply_patch "$file" || true
done < <(find "$REGISTRY_ROOT" -type f -path "*/llama-cpp-sys-4-*/llama.cpp/src/llama-context.cpp" -print0 2>/dev/null)

if [ "$found" -eq 0 ]; then
  echo "No llama-cpp-sys-4 source found. Run 'cargo fetch' in src-tauri first."
fi
