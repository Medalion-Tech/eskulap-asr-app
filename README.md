# Eskulap ASR

Desktop application for Polish medical speech recognition, powered by the [lion-ai/eskulap-asr-turbo-beta](https://huggingface.co/lion-ai/eskulap-asr-turbo-beta) model.

## Features

- Record voice notes directly in the app
- Automatic transcription using a fine-tuned Whisper model optimized for Polish medical speech
- Compile multiple notes into a single text
- Copy to clipboard with one click
- Metal acceleration on Apple Silicon Macs

## Installation

Download the latest release from the [Releases](../../releases) page:

- **macOS**: Download the `.dmg` file, open it, drag the app to Applications.
  - On first launch: right-click the app > "Open" (bypasses Gatekeeper for unsigned apps).
- **Windows**: Download the `.exe` installer and run it.
  - Click "More info" > "Run anyway" if SmartScreen appears.

On first launch, the app will download the ASR model (~800 MB). This only happens once.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) >= 1.77
- [Tauri CLI](https://v2.tauri.app/) (`cargo install tauri-cli`)

### Setup

```bash
npm install
cargo tauri dev
```

### Build

```bash
cargo tauri build
```

## Model Conversion

To convert the HuggingFace model to GGML Q8_0 format:

```bash
pip install torch transformers safetensors huggingface_hub
python scripts/convert-model.py
```

Then upload the resulting `ggml-model-q8_0.bin` to the HuggingFace repo.

## Tech Stack

- **Tauri 2.0** — lightweight desktop framework
- **whisper-rs** — Rust bindings for whisper.cpp
- **Svelte 5** — reactive UI
- **cpal** — cross-platform audio recording

## License

Apache-2.0
