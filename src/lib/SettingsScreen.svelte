<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { screen } from "./stores";

  type AsrStrategy = "greedy" | "beam_search";

  interface AsrSettings {
    language: string;
    translate: boolean;
    threads: number;
    strategy: AsrStrategy;
    greedy_best_of: number;
    beam_size: number;
    temperature: number;
    single_segment: boolean;
    max_text_context: number;
    initial_prompt: string;
  }

  interface LlmSettings {
    model_id: string;
    context_size: number;
    batch_size: number;
    threads: number;
    batch_threads: number;
    max_tokens: number;
    temperature: number;
    top_p: number;
    seed: number;
    flash_attention: boolean;
    kv_cache_enabled: boolean;
  }

  interface AppSettings {
    llm_enabled: boolean;
    asr: AsrSettings;
    llm: LlmSettings;
  }

  interface DlProgress {
    stage: string;
    downloaded: number;
    total: number;
    percent: number;
  }

  interface LlmModelVariant {
    id: string;
    label: string;
    filename: string;
    size_bytes: number;
    downloaded: boolean;
    active: boolean;
  }

  const maxThreads = Math.max(1, Math.min(64, navigator.hardwareConcurrency || 12));

  let saved = $state<AppSettings | null>(null);
  let draft = $state<AppSettings | null>(null);
  let llmModelVariants = $state<LlmModelVariant[]>([]);
  let downloading = $state(false);
  let downloadingModelId = $state<string | null>(null);
  let downloadPercent = $state(0);
  let downloadStage = $state("");
  let saving = $state(false);
  let loading = $state(true);
  let error = $state("");
  let notice = $state("");

  const dirty = $derived(
    saved && draft ? JSON.stringify(saved) !== JSON.stringify(draft) : false
  );

  function clone<T>(value: T): T {
    return JSON.parse(JSON.stringify(value));
  }

  onMount(async () => {
    try {
      saved = await invoke<AppSettings>("get_settings");
      draft = clone(saved);
      llmModelVariants = await invoke<LlmModelVariant[]>("get_llm_model_variants");
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  async function save() {
    if (!draft) return;
    saving = true;
    error = "";
    notice = "";
    try {
      const result = await invoke<{ saved: boolean; llm_unloaded: boolean }>("set_settings", {
        newSettings: draft,
      });
      saved = clone(draft);
      llmModelVariants = await invoke<LlmModelVariant[]>("get_llm_model_variants");
      notice = result.llm_unloaded
        ? "Zapisano. Model LLM został rozładowany."
        : "Zapisano ustawienia.";
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }

  async function restoreDefaults() {
    error = "";
    notice = "";
    try {
      draft = await invoke<AppSettings>("get_default_settings");
    } catch (err) {
      error = String(err);
    }
  }

  function cancel() {
    if (saved) {
      draft = clone(saved);
      error = "";
      notice = "";
    }
  }

  async function downloadLlm(modelId: string) {
    downloading = true;
    downloadingModelId = modelId;
    error = "";
    notice = "";
    downloadPercent = 0;
    try {
      const onProgress = new Channel<DlProgress>();
      onProgress.onmessage = (p) => {
        downloadStage = p.stage;
        downloadPercent = Math.round(p.percent);
      };
      await invoke("download_llm_model_variant", { modelId, onProgress });
      llmModelVariants = await invoke<LlmModelVariant[]>("get_llm_model_variants");
      notice = `Model ${modelId} pobrany.`;
    } catch (err) {
      error = String(err);
    } finally {
      downloading = false;
      downloadingModelId = null;
    }
  }

  async function deleteLlm(modelId: string) {
    if (!confirm(`Usunąć lokalny plik modelu ${modelId}?`)) return;
    error = "";
    notice = "";
    try {
      await invoke("delete_llm_model_variant", { modelId });
      llmModelVariants = await invoke<LlmModelVariant[]>("get_llm_model_variants");
      notice = `Model ${modelId} usunięty.`;
    } catch (err) {
      error = String(err);
    }
  }

  function formatBytes(bytes: number): string {
    const gib = bytes / 1024 / 1024 / 1024;
    return `${gib.toFixed(gib >= 10 ? 1 : 2)} GB`;
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
    {#if loading || !draft}
      <div class="loading">Ładowanie ustawień…</div>
    {:else}
      <section class="section">
        <h2>AI notatek</h2>
        <label class="toggle-row">
          <input type="checkbox" bind:checked={draft.llm_enabled} />
          <span class="toggle-text">
            <span class="toggle-title">Włącz AI notatek</span>
            <span class="toggle-desc">Model Gemma 4 E4B generuje sformatowane notatki z transkrypcji.</span>
          </span>
        </label>

      </section>

      <section class="section">
        <h2>Transkrypcja ASR</h2>
        <div class="grid">
          <label class="field">
            <span>Język</span>
            <select bind:value={draft.asr.language}>
              <option value="pl">Polski</option>
              <option value="auto">Auto</option>
              <option value="en">Angielski</option>
              <option value="de">Niemiecki</option>
              <option value="uk">Ukraiński</option>
            </select>
          </label>
          <label class="field">
            <span>Wątki</span>
            <input type="number" min="1" max={maxThreads} bind:value={draft.asr.threads} />
          </label>
          <label class="field">
            <span>Tryb dekodowania</span>
            <select bind:value={draft.asr.strategy}>
              <option value="greedy">Greedy</option>
              <option value="beam_search">Beam search</option>
            </select>
          </label>
          <label class="field">
            <span>Temperatura</span>
            <input type="number" min="0" max="1" step="0.05" bind:value={draft.asr.temperature} />
          </label>
          <label class="field">
            <span>Best of</span>
            <input type="number" min="1" max="10" bind:value={draft.asr.greedy_best_of} />
          </label>
          <label class="field">
            <span>Beam size</span>
            <input type="number" min="1" max="10" bind:value={draft.asr.beam_size} />
          </label>
          <label class="field">
            <span>Kontekst tekstowy</span>
            <input type="number" min="0" max="4096" bind:value={draft.asr.max_text_context} />
          </label>
        </div>

        <label class="toggle-row compact">
          <input type="checkbox" bind:checked={draft.asr.translate} />
          <span class="toggle-text">
            <span class="toggle-title">Tłumacz na angielski</span>
          </span>
        </label>
        <label class="toggle-row compact">
          <input type="checkbox" bind:checked={draft.asr.single_segment} />
          <span class="toggle-text">
            <span class="toggle-title">Pojedynczy segment</span>
          </span>
        </label>
        <label class="field full">
          <span>Początkowy prompt ASR</span>
          <textarea rows="3" bind:value={draft.asr.initial_prompt}></textarea>
        </label>
      </section>

      <section class="section">
        <h2>Formatowanie LLM</h2>
        <label class="field model-select">
          <span>Model Gemma</span>
          <select bind:value={draft.llm.model_id}>
            {#each llmModelVariants as variant (variant.id)}
              <option value={variant.id}>
                {variant.label} · {formatBytes(variant.size_bytes)}{variant.downloaded ? " · pobrany" : ""}
              </option>
            {/each}
          </select>
        </label>

        <div class="model-list">
          {#each llmModelVariants as variant (variant.id)}
            <div class="model-row" class:selected={draft.llm.model_id === variant.id}>
              <div class="model-info">
                <div class="model-title">
                  <span>{variant.label}</span>
                  {#if draft.llm.model_id === variant.id}
                    <span class="badge badge-active">aktywny</span>
                  {/if}
                  {#if variant.downloaded}
                    <span class="badge badge-ok">pobrany</span>
                  {/if}
                </div>
                <div class="model-meta">{formatBytes(variant.size_bytes)} · {variant.filename}</div>
              </div>
              <div class="model-actions">
                {#if downloading && downloadingModelId === variant.id}
                  <div class="model-progress">
                    <div class="progress-bar">
                      <div class="progress-fill" style="width: {downloadPercent}%"></div>
                    </div>
                    <span class="progress-text">{downloadPercent}% · {downloadStage}</span>
                  </div>
                {:else if variant.downloaded}
                  <button class="btn btn-danger-ghost btn-sm" onclick={() => deleteLlm(variant.id)}>
                    Usuń
                  </button>
                {:else}
                  <button class="btn btn-outline btn-sm" disabled={downloading} onclick={() => downloadLlm(variant.id)}>
                    Pobierz
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        <div class="grid">
          <label class="field">
            <span>Rozmiar kontekstu</span>
            <input type="number" min="1024" max="32768" step="512" bind:value={draft.llm.context_size} />
          </label>
          <label class="field">
            <span>Rozmiar batcha</span>
            <input type="number" min="32" max="4096" step="32" bind:value={draft.llm.batch_size} />
          </label>
          <label class="field">
            <span>Wątki</span>
            <input type="number" min="1" max={maxThreads} bind:value={draft.llm.threads} />
          </label>
          <label class="field">
            <span>Wątki batch</span>
            <input type="number" min="1" max={maxThreads} bind:value={draft.llm.batch_threads} />
          </label>
          <label class="field">
            <span>Nowe tokeny</span>
            <input type="number" min="128" max="4096" step="64" bind:value={draft.llm.max_tokens} />
          </label>
          <label class="field">
            <span>Temperatura</span>
            <input type="number" min="0" max="2" step="0.05" bind:value={draft.llm.temperature} />
          </label>
          <label class="field">
            <span>Top p</span>
            <input type="number" min="0.05" max="1" step="0.05" bind:value={draft.llm.top_p} />
          </label>
          <label class="field">
            <span>Seed</span>
            <input type="number" min="0" step="1" bind:value={draft.llm.seed} />
          </label>
        </div>

        <label class="toggle-row compact">
          <input type="checkbox" bind:checked={draft.llm.flash_attention} />
          <span class="toggle-text">
            <span class="toggle-title">Flash attention</span>
          </span>
        </label>
        <label class="toggle-row compact">
          <input type="checkbox" bind:checked={draft.llm.kv_cache_enabled} />
          <span class="toggle-text">
            <span class="toggle-title">Cache KV prefiksów</span>
          </span>
        </label>
      </section>

    {/if}
  </div>

  {#if draft}
    <footer class="actions">
      <div class="messages">
        {#if error}
          <p class="error">{error}</p>
        {:else if notice}
          <p class="notice">{notice}</p>
        {/if}
      </div>
      <button class="btn btn-ghost" onclick={cancel} disabled={!dirty || saving}>Anuluj</button>
      <button class="btn btn-outline" onclick={restoreDefaults} disabled={saving}>Przywróć domyślne</button>
      <button class="btn btn-solid" onclick={save} disabled={!dirty || saving}>
        {saving ? "Zapisywanie…" : "Zapisz"}
      </button>
    </footer>
  {/if}
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

  .loading {
    padding: 24px 20px;
    color: var(--text-muted);
    font-size: 13px;
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
    margin-bottom: 12px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }

  .field.full {
    margin-top: 12px;
  }

  .field span {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
  }

  input[type="number"],
  select,
  textarea {
    width: 100%;
    min-height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    padding: 5px 8px;
    font: inherit;
    font-size: 13px;
  }

  textarea {
    resize: vertical;
    line-height: 1.4;
  }

  .toggle-row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }

  .toggle-row.compact {
    margin-top: 12px;
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

  .model-select {
    margin-bottom: 12px;
  }

  .model-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 16px;
  }

  .model-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .model-row.selected {
    border-color: var(--accent-soft-border);
    background: var(--accent-soft-bg);
  }

  .model-info {
    min-width: 0;
  }

  .model-title {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .model-meta {
    margin-top: 2px;
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-actions {
    display: flex;
    justify-content: flex-end;
    flex-shrink: 0;
  }

  .model-progress {
    width: 150px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
  }

  .badge-ok {
    background: var(--accent-soft-bg);
    color: var(--accent-text);
  }

  .badge-active {
    background: var(--gray-12);
    color: var(--gray-1);
  }

  .btn-sm {
    height: 26px;
    padding: 4px 10px;
    font-size: 12px;
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

  .actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }

  .messages {
    flex: 1;
    min-width: 0;
  }

  .error,
  .notice {
    font-size: 12px;
    line-height: 1.35;
  }

  .error {
    color: var(--danger);
  }

  .notice {
    color: var(--accent-text);
  }

  @media (max-width: 560px) {
    .grid {
      grid-template-columns: 1fr;
    }

    .actions {
      flex-wrap: wrap;
    }

    .model-row {
      align-items: flex-start;
      flex-direction: column;
    }

    .model-actions,
    .model-progress {
      width: 100%;
    }
  }
</style>
