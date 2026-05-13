<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import {
    isRecording,
    isTranscribing,
    isGenerating,
    generationPreview,
    recordingSeconds,
    transcriptionProgress,
    notes,
    statusMessage,
    selectedTemplateId,
    templates,
    transcribeProgress,
    transcriptionPreview,
    transcriptionDone,
    type Note,
  } from "./stores";
  import type { GenerationResult } from "./ast-types";
  import Waveform from "./Waveform.svelte";

  let timer: ReturnType<typeof setInterval> | null = null;
  let generatingPreviewEl: HTMLDivElement | null = null;

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

    try {
      const audio: number[] = await invoke("stop_recording");

      // Stream transcription in 30-second chunks so partial text appears
      // progressively rather than waiting for the whole recording to process.
      $transcriptionPreview = "";
      $transcribeProgress = 0;
      const onSegment = new Channel<string>();
      onSegment.onmessage = (seg: string) => {
        $transcriptionPreview = $transcriptionPreview
          ? $transcriptionPreview + " " + seg
          : seg;
      };
      const raw: string = await invoke("transcribe_streaming", {
        audio,
        onSegment,
      });

      const rawTrim = raw.trim();
      $isTranscribing = false;

      if (!rawTrim) {
        $transcriptionPreview = "";
        $transcribeProgress = 0;
        $statusMessage = "";
        return;
      }

      const tmplId = $selectedTemplateId;

      if (!tmplId) {
        $transcriptionPreview = "";
        $transcribeProgress = 0;
        const note = await invoke<Note>("add_note", { text: rawTrim });
        $notes = [{ ...note, selected: false }, ...$notes];
        $statusMessage = "";
        return;
      }

      const tmpl = $templates.find((t) => t.id === tmplId);
      if (!tmpl) {
        $transcriptionPreview = "";
        $transcribeProgress = 0;
        const note = await invoke<Note>("add_note", { text: rawTrim });
        $notes = [{ ...note, selected: false }, ...$notes];
        $statusMessage = "";
        return;
      }

      // Show completed transcription until LLM returns first token (TTFS)
      $transcriptionDone = true;
      $isGenerating = true;
      $generationPreview = "";

      let firstToken = true;

      const onToken = new Channel<string>();
      onToken.onmessage = (piece: string) => {
        if (firstToken) {
          firstToken = false;
          $transcriptionDone = false;
          $transcriptionPreview = "";
          $transcribeProgress = 0;
        }
        $generationPreview += piece;
        requestAnimationFrame(() => {
          if (generatingPreviewEl) {
            generatingPreviewEl.scrollTop = generatingPreviewEl.scrollHeight;
          }
        });
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
      $transcriptionDone = false;
      $transcriptionPreview = "";
      $transcribeProgress = 0;
      $transcriptionProgress = 0;
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
      class:waiting={$transcriptionDone}
      disabled={$isTranscribing || $isGenerating || $transcriptionDone}
      onclick={toggleRecording}
      aria-label={$isRecording ? "Zatrzymaj nagrywanie" : "Rozpocznij nagrywanie"}
    >
      {#if $transcriptionDone}
        <div class="check-shape">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        </div>
      {:else if $isTranscribing || $isGenerating}
        {#if $transcribeProgress > 0 && $transcribeProgress < 100}
          <div
            class="progress-ring"
            style="--pct: {$transcribeProgress}%"
            aria-label="Postęp transkrypcji {$transcribeProgress}%"
          ></div>
        {:else}
          <div class="spinner"></div>
        {/if}
      {:else if $isRecording}
        <span class="stop-shape"></span>
      {:else}
        <span class="mic-shape"></span>
      {/if}
    </button>

    <div class="label">
      {#if $isGenerating}
        <span class="label-dim">
          Formatowanie<span class="dots"><span>.</span><span>.</span><span>.</span></span>
        </span>
      {:else if $transcriptionDone}
        <span class="label-dim">Transkrypcja gotowa</span>
      {:else if $isTranscribing}
        {#if $transcribeProgress > 0 && $transcribeProgress < 100}
          <span class="label-dim">Transkrypcja… {$transcribeProgress}%</span>
        {:else}
          <span class="label-dim">Transkrypcja…</span>
        {/if}
      {:else if $isRecording}
        <span class="time">{formatTime($recordingSeconds)}</span>
      {:else}
        <span class="label-dim">Naciśnij, aby nagrać</span>
      {/if}
    </div>

    {#if $isTranscribing}
      <div class="progress-bar">
        <div class="progress-fill" style="width: {$transcriptionProgress}%"></div>
      </div>
    {/if}
  </div>

  {#if $transcriptionDone && $transcriptionPreview}
    <div class="preview preview-done">
      <div class="preview-label">
        <span class="done-check">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        </span>
        Transkrypcja zakończona
      </div>
      <pre class="preview-text">{$transcriptionPreview}</pre>
    </div>
  {/if}

  {#if $isTranscribing && $transcriptionPreview}
    <div class="preview">
      <div class="preview-label">Transkrypcja (na bieżąco):</div>
      <pre class="preview-text">{$transcriptionPreview}</pre>
    </div>
  {/if}

  {#if $isGenerating && $generationPreview}
    <div class="preview preview-generating" bind:this={generatingPreviewEl}>
      <div class="preview-label">
        <span class="generating-dot"></span>
        Formatowanie notatki
      </div>
      <pre class="preview-text">{$generationPreview}<span class="cursor"></span></pre>
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

  /* Conic-gradient progress ring — shows whisper.cpp % completion.
     --pct is set inline from $transcribeProgress. */
  .progress-ring {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: conic-gradient(
      var(--accent) var(--pct, 0%),
      var(--gray-5) var(--pct, 0%)
    );
    mask: radial-gradient(farthest-side, transparent 62%, #000 63%);
    -webkit-mask: radial-gradient(farthest-side, transparent 62%, #000 63%);
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

  .progress-bar {
    width: 100%;
    max-width: 200px;
    height: 4px;
    background: var(--gray-4);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 4px;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.2s ease-out;
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

  /* Waiting state after transcription completes */
  .record-btn.waiting {
    border-color: var(--accent);
    background: var(--accent-soft-bg);
    animation: waiting-pulse 1.2s ease-in-out infinite;
  }

  .check-shape {
    width: 16px;
    height: 16px;
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  @keyframes waiting-pulse {
    0%, 100% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 25%, transparent);
    }
    50% {
      box-shadow: 0 0 0 8px color-mix(in srgb, var(--accent) 0%, transparent);
    }
  }

  /* Animated dots on formatting label */
  .dots span {
    animation: dot-blink 1.4s infinite both;
    opacity: 0.2;
  }
  .dots span:nth-child(1) { animation-delay: 0s; }
  .dots span:nth-child(2) { animation-delay: 0.2s; }
  .dots span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes dot-blink {
    0%, 80%, 100% { opacity: 0.2; }
    40% { opacity: 1; }
  }

  /* Completed transcription preview */
  .preview-done {
    border-color: var(--accent-soft-border);
    background: var(--accent-soft-bg);
    animation: preview-done-in 0.3s ease-out;
  }

  .done-check {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--accent);
    color: white;
    margin-right: 6px;
    vertical-align: middle;
    animation: check-pop 0.35s cubic-bezier(0.175, 0.885, 0.32, 1.275);
  }

  @keyframes preview-done-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes check-pop {
    0% {
      transform: scale(0);
      opacity: 0;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }

  /* Generating preview with glow and cursor */
  .preview-generating {
    border-color: var(--accent-soft-border);
    animation: generating-glow 2s ease-in-out infinite;
  }

  @keyframes generating-glow {
    0%, 100% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 8%, transparent);
    }
    50% {
      box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 0%, transparent);
    }
  }

  .cursor {
    display: inline-block;
    width: 2px;
    height: 1.2em;
    background: var(--accent);
    margin-left: 1px;
    vertical-align: text-bottom;
    animation: cursor-blink 1s step-end infinite;
  }

  @keyframes cursor-blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .generating-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    margin-right: 8px;
    vertical-align: middle;
    animation: generating-dot-pulse 1.5s ease-in-out infinite;
  }

  @keyframes generating-dot-pulse {
    0%, 100% {
      transform: scale(1);
      opacity: 0.6;
    }
    50% {
      transform: scale(1.4);
      opacity: 1;
    }
  }
</style>
