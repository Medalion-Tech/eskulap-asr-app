#!/usr/bin/env python3
"""
Convert lion-ai/eskulap-asr-turbo-beta from HuggingFace to GGML format,
then quantize to Q8_0 for use with whisper.cpp / whisper-rs.

Prerequisites:
  pip install torch transformers safetensors huggingface_hub

Usage:
  1. Clone whisper.cpp: git clone https://github.com/ggerganov/whisper.cpp
  2. Build the quantize tool: cd whisper.cpp && make quantize
  3. Run this script: python convert-model.py
  4. Upload the resulting .bin to HuggingFace

The script will:
  - Download the model from HuggingFace
  - Convert to GGML format using whisper.cpp's converter
  - Quantize to Q8_0
"""

import subprocess
import sys
from pathlib import Path

MODEL_ID = "lion-ai/eskulap-asr-turbo-beta"
WHISPER_CPP_DIR = Path("whisper.cpp")
OUTPUT_DIR = Path("output")


def main():
    OUTPUT_DIR.mkdir(exist_ok=True)

    # Step 1: Ensure whisper.cpp is available
    if not WHISPER_CPP_DIR.exists():
        print("Cloning whisper.cpp...")
        subprocess.run(
            ["git", "clone", "https://github.com/ggerganov/whisper.cpp"],
            check=True,
        )

    # Step 2: Build quantize tool
    quantize_bin = WHISPER_CPP_DIR / "build" / "bin" / "quantize"
    if not quantize_bin.exists():
        print("Building whisper.cpp tools...")
        build_dir = WHISPER_CPP_DIR / "build"
        build_dir.mkdir(exist_ok=True)
        subprocess.run(
            ["cmake", ".."],
            cwd=build_dir,
            check=True,
        )
        subprocess.run(
            ["cmake", "--build", ".", "--config", "Release"],
            cwd=build_dir,
            check=True,
        )

    # Step 3: Download model from HuggingFace
    model_dir = OUTPUT_DIR / "hf-model"
    if not model_dir.exists():
        print(f"Downloading {MODEL_ID}...")
        from huggingface_hub import snapshot_download

        snapshot_download(
            repo_id=MODEL_ID,
            local_dir=str(model_dir),
        )

    # Step 4: Convert to GGML
    convert_script = WHISPER_CPP_DIR / "models" / "convert-hf-to-ggml.py"
    ggml_output = OUTPUT_DIR / "ggml-model.bin"

    if not ggml_output.exists():
        print("Converting HF model to GGML format...")
        subprocess.run(
            [
                sys.executable,
                str(convert_script),
                str(model_dir),
                str(OUTPUT_DIR),
            ],
            check=True,
        )
        # The script outputs to OUTPUT_DIR/ggml-model.bin

    # Step 5: Quantize to Q8_0
    q8_output = OUTPUT_DIR / "ggml-model-q8_0.bin"
    if not q8_output.exists():
        print("Quantizing to Q8_0...")
        subprocess.run(
            [str(quantize_bin), str(ggml_output), str(q8_output), "q8_0"],
            check=True,
        )

    print(f"\nDone! Quantized model saved to: {q8_output}")
    print(f"Size: {q8_output.stat().st_size / 1024 / 1024:.1f} MB")
    print(
        "\nUpload this file to HuggingFace:"
    )
    print(
        f"  huggingface-cli upload {MODEL_ID} {q8_output} ggml-model-q8_0.bin"
    )


if __name__ == "__main__":
    main()
