<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { screen } from "./stores";
  import TemplateList from "./TemplateList.svelte";

  let llmEnabled = $state(false);
  let llmModelPresent = $state(false);
  let downloading = $state(false);
  let downloadPercent = $state(0);
  let downloadStage = $state("");
  let busyToggle = $state(false);
  let error = $state("");

  interface DlProgress {
    stage: string;
    downloaded: number;
    total: number;
    percent: number;
  }

  onMount(async () => {
    try {
      const settings = await invoke<{ llm_enabled: boolean }>("get_settings");
      llmEnabled = settings.llm_enabled;
      llmModelPresent = await invoke<boolean>("check_llm_model_exists");
    } catch (e) {
      error = String(e);
    }
  });

  async function persist() {
    await invoke("set_settings", { newSettings: { llm_enabled: llmEnabled } });
  }

  async function onToggleLlm(e: Event) {
    const target = e.target as HTMLInputElement;
    const next = target.checked;
    busyToggle = true;
    error = "";
    try {
      llmEnabled = next;
      await persist();
      if (!next) {
        await invoke("unload_llm_model");
      }
    } catch (err) {
      error = String(err);
      llmEnabled = !next;
    } finally {
      busyToggle = false;
    }
  }

  async function downloadLlm() {
    downloading = true;
    error = "";
    downloadPercent = 0;
    try {
      const onProgress = new Channel<DlProgress>();
      onProgress.onmessage = (p) => {
        downloadStage = p.stage;
        downloadPercent = Math.round(p.percent);
      };
      await invoke("download_llm_model", { onProgress });
      llmModelPresent = true;
    } catch (err) {
      error = String(err);
    } finally {
      downloading = false;
    }
  }

  function back() {
    $screen = "main";
  }
</script>

<div class="settings">
  <header class="header">
    <button class="back-btn" onclick={back} aria-label="Wróć">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <h1>Ustawienia</h1>
    <div class="spacer"></div>
  </header>

  <div class="body">
    <section class="section">
      <h2>AI streszczenia notatek</h2>
      <label class="toggle-row">
        <input
          type="checkbox"
          checked={llmEnabled}
          disabled={busyToggle}
          onchange={onToggleLlm}
        />
        <span class="toggle-text">
          <span class="toggle-title">Włączone</span>
          <span class="toggle-desc">
            Model Gemma 4 E4B generuje sformatowane notatki z transkrypcji.
          </span>
        </span>
      </label>

      {#if llmEnabled}
        <div class="model-status">
          {#if llmModelPresent}
            <span class="badge badge-ok">✓ Model pobrany</span>
          {:else if downloading}
            <div class="progress">
              <div class="progress-bar">
                <div class="progress-fill" style="width: {downloadPercent}%"></div>
              </div>
              <span class="progress-text">{downloadPercent}% · {downloadStage}</span>
            </div>
          {:else}
            <button class="btn btn-solid" onclick={downloadLlm}>
              Pobierz model (~5 GB)
            </button>
          {/if}
        </div>
      {/if}

      {#if error}
        <p class="error">{error}</p>
      {/if}
    </section>

    <TemplateList />
  </div>
</div>

<style>
  .settings {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 16px 6px 84px;
    height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    -webkit-app-region: drag;
    app-region: drag;
    flex-shrink: 0;
  }

  .back-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    -webkit-app-region: no-drag;
    app-region: no-drag;
    transition: background var(--duration-fast) var(--easing);
  }

  .back-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  h1 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .spacer {
    flex: 1;
  }

  .body {
    flex: 1;
    overflow-y: auto;
  }

  .section {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .section h2 {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    margin-bottom: 10px;
  }

  .toggle-row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }

  .toggle-row input {
    margin-top: 2px;
  }

  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .toggle-title {
    font-size: 13px;
    color: var(--text);
    font-weight: 500;
  }

  .toggle-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .model-status {
    margin-top: 12px;
    padding-left: 24px;
  }

  .badge {
    display: inline-block;
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    font-weight: 500;
  }

  .badge-ok {
    background: var(--accent-soft-bg);
    color: var(--accent-text);
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 340px;
  }

  .progress-bar {
    height: 4px;
    background: var(--gray-4);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 300ms var(--easing);
  }

  .progress-text {
    font-size: 11px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .error {
    color: var(--danger);
    font-size: 12px;
    margin-top: 10px;
  }
</style>
