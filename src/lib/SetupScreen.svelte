<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { screen, downloadProgress, statusMessage, notes } from "./stores";
  import { MODEL_REPO, MODEL_QUANTIZATION } from "./model-info";

  let downloading = $state(false);
  let error = $state("");

  async function handleDownload() {
    downloading = true;
    error = "";
    $statusMessage = "Pobieranie modelu…";

    try {
      const onProgress = new Channel<{
        downloaded: number;
        total: number;
        percent: number;
      }>();
      onProgress.onmessage = (progress) => {
        $downloadProgress = Math.round(progress.percent);
      };

      await invoke("download_model", { onProgress });
      $downloadProgress = 100;
      $statusMessage = "Ładowanie modelu…";

      await invoke("load_model");

      const savedNotes = await invoke<
        Array<{ id: string; timestamp: string; text: string }>
      >("get_notes");
      $notes = savedNotes.map((n) => ({ ...n, selected: false }));

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
    <p class="subtitle">Transkrypcja mowy medycznej</p>

    <a
      class="model-link"
      href="https://huggingface.co/{MODEL_REPO}"
      target="_blank"
      rel="noreferrer"
    >
      <span class="model-name">{MODEL_REPO}</span>
      <span class="model-quant">{MODEL_QUANTIZATION}</span>
    </a>

    <div class="action">
      {#if !downloading}
        <button class="btn btn-solid download-btn" onclick={handleDownload}>
          Pobierz model
        </button>
        <p class="hint">Około 800 MB · jednorazowo</p>
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
    margin-bottom: 16px;
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
    margin-bottom: 20px;
    letter-spacing: -0.005em;
  }

  .model-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-muted);
    text-decoration: none;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-subtle);
    margin-bottom: 36px;
    transition: border-color var(--duration-fast) var(--easing),
      color var(--duration-fast) var(--easing);
  }

  .model-link:hover {
    border-color: var(--border-strong);
    color: var(--text-secondary);
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
    width: 320px;
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
