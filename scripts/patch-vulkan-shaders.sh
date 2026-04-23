#!/usr/bin/env bash
# Patches ggml-vulkan's ExternalProject_Add to solve TWO issues that break
# Windows CI Vulkan builds of whisper-rs-sys / llama-cpp-sys-4:
#
# Issue 1: No CMAKE_C_COMPILER could be found.
#   ExternalProject_Add does not forward CMAKE_C_COMPILER / CMAKE_CXX_COMPILER
#   from the parent cmake invocation to the child cmake. The parent (invoked by
#   Rust's cmake-rs) detects cl.exe via vswhere/registry through cc-rs, but the
#   child runs fresh and cannot locate it, failing with "No CMAKE_C_COMPILER
#   could be found."
#   Fix: inject -DCMAKE_C_COMPILER=${CMAKE_C_COMPILER} and
#        -DCMAKE_CXX_COMPILER=${CMAKE_CXX_COMPILER} into the child's CMAKE_ARGS.
#
# Issue 2: C1083 "Cannot open compiler generated file: '': Invalid argument".
#   When the child uses the Visual Studio generator (the default on Windows),
#   its TryCompile step generates an auto-created cmTC_XXX.vcxproj. Because
#   compiler identification chicken-and-eggs (child has no PlatformToolset
#   context yet), that vcxproj is generated WITHOUT a PlatformToolset. MSBuild
#   then builds it without VC-targets setting INCLUDE / LIB / LIBPATH for
#   cl.exe. cl.exe launches with empty env → fails to construct intermediate
#   file paths → C1083 with an empty filename.
#   Fix: override CHILD generator to Ninja via `CMAKE_GENERATOR "Ninja"`.
#   Ninja invokes cl.exe DIRECTLY with the parent process env (no MSBuild
#   layer, no vcxproj, no PlatformToolset dependency). This matches what
#   upstream llama.cpp uses for its non-Vulkan Windows CI jobs to avoid
#   exactly this class of MSBuild/ExternalProject interaction bug.
#   Parent cmake stays on VS generator — this only affects the child
#   sub-project (vulkan-shaders-gen, a small host tool that generates
#   GLSL shader code at build time).
#   Ninja is pre-installed on GitHub Actions windows-latest (windows-2022)
#   at C:\ProgramData\Chocolatey\bin\ninja.exe and reachable via PATH.
#
# Applies to both whisper-rs-sys and llama-cpp-sys-4 crates (both vendor
# whisper.cpp/llama.cpp which include the same ggml-vulkan CMakeLists.txt).
# Idempotent: each of the two patches is checked independently, so the
# script is safe to re-run and works on files that already had a subset
# of the patches applied (e.g. from an earlier script version).

set -euo pipefail

REGISTRY_ROOT="${CARGO_HOME:-$HOME/.cargo}/registry/src"

if [ ! -d "$REGISTRY_ROOT" ]; then
  echo "Cargo registry not found at $REGISTRY_ROOT; run 'cargo fetch' first."
  exit 0
fi

# Patch A: force Ninja generator on the child ExternalProject.
# Anchor: the `SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/vulkan-shaders` line
# which appears exactly once inside ExternalProject_Add(vulkan-shaders-gen ...).
patch_generator() {
  local file=$1
  if grep -q 'CMAKE_GENERATOR "Ninja"' "$file"; then
    echo "  generator patch already applied"
    return 0
  fi
  if ! grep -q -- 'SOURCE_DIR \${CMAKE_CURRENT_SOURCE_DIR}/vulkan-shaders' "$file"; then
    echo "  WARNING: SOURCE_DIR anchor not found (upstream changed?) — skipping generator patch"
    return 0
  fi
  sed -i.bak '/SOURCE_DIR \${CMAKE_CURRENT_SOURCE_DIR}\/vulkan-shaders/a\
        CMAKE_GENERATOR "Ninja"                                # PATCHED: bypass MSBuild TryCompile for child (forward compiler to ExternalProject)' "$file"
  rm -f "${file}.bak"
  echo "  generator patch applied"
}

# Patch B: forward CMAKE_C_COMPILER / CMAKE_CXX_COMPILER from parent.
# Anchor: the `-DCMAKE_BUILD_TYPE=$<CONFIG>` line inside CMAKE_ARGS.
patch_compiler() {
  local file=$1
  if grep -q -- '-DCMAKE_C_COMPILER=\${CMAKE_C_COMPILER}' "$file"; then
    echo "  compiler patch already applied"
    return 0
  fi
  if ! grep -q -- '-DCMAKE_BUILD_TYPE=\$<CONFIG>' "$file"; then
    echo "  WARNING: CMAKE_BUILD_TYPE anchor not found (upstream changed?) — skipping compiler patch"
    return 0
  fi
  sed -i.bak '/\-DCMAKE_BUILD_TYPE=\$<CONFIG>/a\
                   -DCMAKE_C_COMPILER=${CMAKE_C_COMPILER}     # PATCHED: forward compiler to ExternalProject\
                   -DCMAKE_CXX_COMPILER=${CMAKE_CXX_COMPILER} # PATCHED: forward compiler to ExternalProject' "$file"
  rm -f "${file}.bak"
  echo "  compiler patch applied"
}

patch_file() {
  local file=$1
  echo "Patching: $file"
  patch_generator "$file"
  patch_compiler "$file"
}

found=0
while IFS= read -r -d '' file; do
  found=$((found + 1))
  patch_file "$file"
done < <(find "$REGISTRY_ROOT" -type f -path "*/ggml/src/ggml-vulkan/CMakeLists.txt" -print0 2>/dev/null)

if [ "$found" -eq 0 ]; then
  echo "No ggml-vulkan CMakeLists.txt found. Run 'cargo fetch' in src-tauri first."
fi

echo "Processed $found ggml-vulkan cmake file(s)."
