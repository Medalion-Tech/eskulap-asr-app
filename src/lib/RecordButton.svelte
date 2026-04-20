<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import {
    isRecording,
    isTranscribing,
    isGenerating,
    generationPreview,
    recordingSeconds,
    notes,
    statusMessage,
    selectedTemplateId,
    templates,
    type Note,
  } from "./stores";
  import type { GenerationResult } from "./ast-types";
  import Waveform from "./Waveform.svelte";

  let timer: ReturnType<typeof setInterval> | null = null;

  function formatTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function toggleRecording() {
    if ($isTranscribing || $isGenerating) return;
    if ($isRecording) {
      await stopRecording();
    } else {
      await startRecording();
    }
  }

  async function startRecording() {
    try {
      await invoke("start_recording");
      $isRecording = true;
      $recordingSeconds = 0;
      timer = setInterval(() => {
        $recordingSeconds++;
      }, 1000);
    } catch (e: any) {
      $statusMessage = `Błąd nagrywania: ${e}`;
    }
  }

  async function stopRecording() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
    $isRecording = false;
    $isTranscribing = true;
    $statusMessage = "Transkrypcja…";

    try {
      const audio: number[] = await invoke("stop_recording");
      const raw: string = await invoke("transcribe", { audio });
      const rawTrim = raw.trim();
      $isTranscribing = false;

      if (!rawTrim) {
        $statusMessage = "";
        return;
      }

      const tmplId = $selectedTemplateId;

      if (!tmplId) {
        const note = await invoke<Note>("add_note", { text: rawTrim });
        $notes = [{ ...note, selected: false }, ...$notes];
        $statusMessage = "";
        return;
      }

      const tmpl = $templates.find((t) => t.id === tmplId);
      if (!tmpl) {
        const note = await invoke<Note>("add_note", { text: rawTrim });
        $notes = [{ ...note, selected: false }, ...$notes];
        $statusMessage = "";
        return;
      }

      $isGenerating = true;
      $generationPreview = "";
      $statusMessage = `Formatowanie – ${tmpl.name}…`;

      const onToken = new Channel<string>();
      onToken.onmessage = (piece: string) => {
        $generationPreview += piece;
      };

      const result: GenerationResult = await invoke("generate_from_template", {
        templateId: tmplId,
        rawTranscription: rawTrim,
        onToken,
      });

      const note = await invoke<Note>("add_note_with_template", {
        text: result.display_text,
        rawTranscription: rawTrim,
        templateId: tmplId,
        templateName: tmpl.name,
        filled: result.filled,
        rawLlmOutput: result.raw_output,
      });
      $notes = [{ ...note, selected: false }, ...$notes];

      if (result.parse_quality_low) {
        $statusMessage = `Uwaga: parser rozpoznał ${result.parsed_ok}/${result.total_slots} slotów. Sprawdź notatkę.`;
      }

      $statusMessage = "";
    } catch (e: any) {
      $statusMessage = `Błąd: ${e}`;
    } finally {
      $isTranscribing = false;
      $isGenerating = false;
      $generationPreview = "";
    }
  }
</script>

<section class="recorder">
  <div class="waveform-container" class:active={$isRecording}>
    <Waveform active={$isRecording} />
  </div>

  <div class="controls">
    <button
      class="record-btn"
      class:recording={$isRecording}
      disabled={$isTranscribing || $isGenerating}
      onclick={toggleRecording}
      aria-label={$isRecording ? "Zatrzymaj nagrywanie" : "Rozpocznij nagrywanie"}
    >
      {#if $isTranscribing || $isGenerating}
        <div class="spinner"></div>
      {:else if $isRecording}
        <span class="stop-shape"></span>
      {:else}
        <span class="mic-shape"></span>
      {/if}
    </button>

    <div class="label">
      {#if $isGenerating}
        <span class="label-dim">Formatowanie…</span>
      {:else if $isTranscribing}
        <span class="label-dim">Transkrypcja…</span>
      {:else if $isRecording}
        <span class="time">{formatTime($recordingSeconds)}</span>
      {:else}
        <span class="label-dim">Naciśnij, aby nagrać</span>
      {/if}
    </div>
  </div>

  {#if $isGenerating && $generationPreview}
    <div class="preview">
      <div class="preview-label">Model generuje notatkę:</div>
      <pre class="preview-text">{$generationPreview}</pre>
    </div>
  {/if}
</section>

<style>
  .recorder {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px 24px 16px;
    gap: 14px;
  }

  .waveform-container {
    width: 100%;
    max-width: 560px;
    height: 64px;
    opacity: 0.35;
    transition: opacity var(--duration-base) var(--easing);
  }

  .waveform-container.active {
    opacity: 1;
  }

  .controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .record-btn {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-sm);
    transition: transform var(--duration-fast) var(--easing),
      border-color var(--duration-fast) var(--easing),
      box-shadow var(--duration-fast) var(--easing);
  }

  .record-btn:hover:not(:disabled) {
    transform: scale(1.05);
    box-shadow: var(--shadow-md);
    border-color: var(--accent);
  }

  .record-btn:active:not(:disabled) {
    transform: scale(0.96);
  }

  .record-btn.recording {
    border-color: var(--danger);
    background: var(--surface);
    animation: record-pulse 1.8s ease-in-out infinite;
  }

  @keyframes record-pulse {
    0%, 100% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--danger) 28%, transparent);
    }
    50% {
      box-shadow: 0 0 0 12px color-mix(in srgb, var(--danger) 0%, transparent);
    }
  }

  .mic-shape {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
    transition: background var(--duration-fast) var(--easing);
  }

  .record-btn:hover:not(:disabled) .mic-shape {
    background: var(--accent-hover);
  }

  .stop-shape {
    width: 14px;
    height: 14px;
    border-radius: 2px;
    background: var(--danger);
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--gray-5);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .label {
    min-height: 18px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
  }

  .label-dim {
    color: var(--text-muted);
  }

  .time {
    color: var(--accent-text);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }

  .preview {
    width: 100%;
    max-width: 560px;
    max-height: 160px;
    overflow-y: auto;
    padding: 10px 12px;
    background: var(--bg-subtle);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .preview-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .preview-text {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
  }
</style>
