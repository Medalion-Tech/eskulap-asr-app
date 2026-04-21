#!/usr/bin/env bash
# Patches ggml-vulkan's ExternalProject_Add to forward CMAKE_C_COMPILER and
# CMAKE_CXX_COMPILER to the vulkan-shaders-gen sub-project.
#
# Without this, the sub-project spawns a fresh cmake configure that cannot
# locate the MSVC compiler on Windows CI runners. The parent cmake (using the
# Visual Studio generator) detects cl.exe via the Windows registry, but
# ExternalProject_Add does NOT propagate this to the child cmake's CMAKE_ARGS.
# The child falls back to PATH-based detection and fails with:
#   "No CMAKE_C_COMPILER could be found."
#
# This patch adds two lines to ExternalProject_Add so the child cmake receives
# the compiler paths from the parent — works regardless of generator or OS.
#
# Applies to both whisper-rs-sys and llama-cpp-sys-4 crates.
# Idempotent: re-running the script no-ops when already patched.

set -euo pipefail

REGISTRY_ROOT="${CARGO_HOME:-$HOME/.cargo}/registry/src"

if [ ! -d "$REGISTRY_ROOT" ]; then
  echo "Cargo registry not found at $REGISTRY_ROOT; run 'cargo fetch' first."
  exit 0
fi

patch_file() {
  local file=$1
  local marker="PATCHED: forward compiler to ExternalProject"

  if grep -q "$marker" "$file" 2>/dev/null; then
    echo "Already patched: $file"
    return 0
  fi

  # Match the line: -DCMAKE_BUILD_TYPE=$<CONFIG>
  # and append two more -D lines after it to forward compilers.
  if grep -q -- '-DCMAKE_BUILD_TYPE=\$<CONFIG>' "$file" 2>/dev/null; then
    sed -i.bak '/\-DCMAKE_BUILD_TYPE=\$<CONFIG>/a\
                   -DCMAKE_C_COMPILER=${CMAKE_C_COMPILER}   # PATCHED: forward compiler to ExternalProject\
                   -DCMAKE_CXX_COMPILER=${CMAKE_CXX_COMPILER} # PATCHED: forward compiler to ExternalProject' "$file"
    rm -f "${file}.bak"
    echo "Patched: $file"
    return 0
  fi

  echo "WARNING: ExternalProject pattern not found in $file (upstream changed?)"
  return 0
}

found=0
while IFS= read -r -d '' file; do
  found=$((found + 1))
  patch_file "$file"
done < <(find "$REGISTRY_ROOT" -type f -path "*/ggml/src/ggml-vulkan/CMakeLists.txt" -print0 2>/dev/null)

if [ "$found" -eq 0 ]; then
  echo "No ggml-vulkan CMakeLists.txt found. Run 'cargo fetch' in src-tauri first."
fi

echo "Patched $found ggml-vulkan cmake file(s)."
