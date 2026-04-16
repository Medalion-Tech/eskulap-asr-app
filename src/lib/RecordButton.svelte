<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    isRecording,
    isTranscribing,
    recordingSeconds,
    notes,
    statusMessage,
  } from "./stores";
  import Waveform from "./Waveform.svelte";

  let timer: ReturnType<typeof setInterval> | null = null;

  function formatTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function toggleRecording() {
    if ($isTranscribing) return;
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
      const text: string = await invoke("transcribe", { audio });

      if (text.trim()) {
        const note = await invoke<{
          id: string;
          timestamp: string;
          text: string;
        }>("add_note", { text: text.trim() });
        $notes = [{ ...note, selected: false }, ...$notes];
      }

      $statusMessage = "";
    } catch (e: any) {
      $statusMessage = `Błąd: ${e}`;
    } finally {
      $isTranscribing = false;
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
      disabled={$isTranscribing}
      onclick={toggleRecording}
      aria-label={$isRecording ? "Zatrzymaj nagrywanie" : "Rozpocznij nagrywanie"}
    >
      {#if $isTranscribing}
        <div class="spinner"></div>
      {:else if $isRecording}
        <span class="stop-shape"></span>
      {:else}
        <span class="mic-shape"></span>
      {/if}
    </button>

    <div class="label">
      {#if $isTranscribing}
        <span class="label-dim">Transkrypcja…</span>
      {:else if $isRecording}
        <span class="time">{formatTime($recordingSeconds)}</span>
      {:else}
        <span class="label-dim">Naciśnij, aby nagrać</span>
      {/if}
    </div>
  </div>
</section>

<style>
  .recorder {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 28px 24px 20px;
    gap: 16px;
  }

  .waveform-container {
    width: 100%;
    max-width: 560px;
    height: 72px;
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
</style>
