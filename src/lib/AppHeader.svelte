<script lang="ts">
  import { MODEL_REPO, MODEL_QUANTIZATION } from "./model-info";
</script>

<header class="app-header">
  <div class="title-group">
    <span class="app-name">Eskulap ASR</span>
  </div>
  <a
    class="model-info"
    href="https://huggingface.co/{MODEL_REPO}"
    target="_blank"
    rel="noreferrer"
    title="Otwórz stronę modelu na HuggingFace"
  >
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="16" x2="12" y2="12" />
      <line x1="12" y1="8" x2="12.01" y2="8" />
    </svg>
    <span class="model-name">{MODEL_REPO}</span>
    <span class="model-quant">{MODEL_QUANTIZATION}</span>
  </a>
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 16px 6px 84px;
    height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    -webkit-app-region: drag;
    app-region: drag;
    flex-shrink: 0;
  }

  .title-group {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .app-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .model-info {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-muted);
    text-decoration: none;
    padding: 3px 8px;
    border-radius: 5px;
    transition: background var(--duration-fast) var(--easing),
      color var(--duration-fast) var(--easing);
    -webkit-app-region: no-drag;
    app-region: no-drag;
    font-family: "Inter Variable", "Inter", sans-serif;
  }

  .model-info:hover {
    background: var(--bg-hover);
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
</style>
