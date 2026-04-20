<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import {
    screen,
    downloadProgress,
    downloadStage,
    statusMessage,
    notes,
    templates,
  } from "./stores";

  let downloading = $state(false);
  let error = $state("");

  interface DlProgress {
    stage: string;
    downloaded: number;
    total: number;
    percent: number;
  }

  async function runDownloadStage(
    cmd: string,
    label: string
  ): Promise<void> {
    $downloadProgress = 0;
    $statusMessage = label;
    const onProgress = new Channel<DlProgress>();
    onProgress.onmessage = (p) => {
      $downloadStage = p.stage;
      $downloadProgress = Math.round(p.percent);
    };
    await invoke(cmd, { onProgress });
    $downloadProgress = 100;
  }

  async function handleDownload() {
    downloading = true;
    error = "";

    try {
      const whisperExists: boolean = await invoke("check_model_exists");
      const llmExists: boolean = await invoke("check_llm_model_exists");

      const stages: Array<{ cmd: string; label: string }> = [];
      if (!whisperExists) {
        stages.push({
          cmd: "download_model",
          label: "Pobieranie modelu ASR…",
        });
      }
      if (!llmExists) {
        stages.push({
          cmd: "download_llm_model",
          label: "Pobieranie modelu LLM…",
        });
      }

      for (let i = 0; i < stages.length; i++) {
        const s = stages[i];
        const label =
          stages.length > 1 ? `${s.label} (${i + 1}/${stages.length})` : s.label;
        await runDownloadStage(s.cmd, label);
      }

      $statusMessage = "Ładowanie modelu ASR…";
      await invoke("load_model");

      $statusMessage = "Ładowanie modelu LLM…";
      await invoke("load_llm_model");

      const savedNotes = await invoke<
        Array<{
          id: string;
          timestamp: string;
          text: string;
          raw_transcription?: string | null;
          template_id?: string | null;
          template_name?: string | null;
        }>
      >("get_notes");
      $notes = savedNotes.map((n) => ({ ...n, selected: false }));
      $templates = await invoke("get_templates");

      $statusMessage = "";
      $screen = "main";
    } catch (e: any) {
      error = e?.toString() || "Pobieranie nie powiodło się";
      $statusMessage = "";
      downloading = false;
    }
  }
</script>

<div class="setup">
  <div class="titlebar-spacer"></div>
  <div class="content">
    <div class="logo">
      <svg width="44" height="44" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/>
        <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
        <line x1="12" y1="19" x2="12" y2="22"/>
      </svg>
    </div>
    <h1>Eskulap ASR</h1>
    <p class="subtitle">Transkrypcja mowy medycznej + szablony notatek</p>

    <div class="models">
      <div class="model-row">
        <span class="model-name">lion-ai/eskulap-asr-turbo-beta</span>
        <span class="model-quant">Q8_0 · ~800 MB</span>
      </div>
      <div class="model-row">
        <span class="model-name">unsloth/gemma-4-E4B-it-GGUF</span>
        <span class="model-quant">Q4_K_M · ~5 GB</span>
      </div>
    </div>

    <div class="action">
      {#if !downloading}
        <button class="btn btn-solid download-btn" onclick={handleDownload}>
          Pobierz modele
        </button>
        <p class="hint">Łącznie ok. 5,8 GB · jednorazowo</p>
      {:else}
        <div class="progress-container">
          <div class="progress-bar">
            <div
              class="progress-fill"
              style="width: {$downloadProgress}%"
            ></div>
          </div>
          <p class="progress-text">
            {#if $downloadProgress < 100}
              {$downloadProgress}% · {$statusMessage}
            {:else}
              {$statusMessage}
            {/if}
          </p>
        </div>
      {/if}
    </div>

    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>
</div>

<style>
  .setup {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .titlebar-spacer {
    height: 28px;
    -webkit-app-region: drag;
    app-region: drag;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 24px;
  }

  .logo {
    color: var(--accent);
    margin-bottom: 14px;
  }

  h1 {
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.025em;
    color: var(--text);
    margin-bottom: 6px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 14px;
    margin-bottom: 18px;
    letter-spacing: -0.005em;
  }

  .models {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 28px;
  }

  .model-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-muted);
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  .model-name {
    font-variant-numeric: tabular-nums;
  }

  .model-quant {
    padding: 1px 5px;
    background: var(--accent-soft-bg);
    color: var(--accent-text);
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .action {
    min-height: 80px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .download-btn {
    padding: 0 24px;
    height: 36px;
    font-size: 14px;
  }

  .hint {
    color: var(--text-muted);
    font-size: 12px;
    margin-top: 14px;
  }

  .progress-container {
    width: 340px;
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
    margin-top: 12px;
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .error {
    color: var(--danger);
    font-size: 13px;
    margin-top: 20px;
    max-width: 360px;
    line-height: 1.5;
  }
</style>
