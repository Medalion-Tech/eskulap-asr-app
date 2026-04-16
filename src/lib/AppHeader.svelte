<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { MODEL_REPO, MODEL_QUANTIZATION } from "./model-info";

  interface AcceleratorInfo {
    backend: string;
    platform: string;
    arch: string;
    threads: number;
    cpu_model: string;
  }

  let accel: AcceleratorInfo | null = $state(null);

  onMount(async () => {
    try {
      accel = await invoke<AcceleratorInfo>("get_accelerator_info");
    } catch (e) {
      console.error("Failed to get accelerator info:", e);
    }
  });

  const isGpu = $derived(accel?.backend === "Metal");
  const tooltip = $derived(
    accel
      ? `${accel.platform} ${accel.arch}${accel.cpu_model ? ` · ${accel.cpu_model}` : ""} · ${accel.threads} wątków`
      : ""
  );
</script>

<header class="app-header">
  <div class="title-group">
    <span class="app-name">Eskulap ASR</span>
  </div>
  <div class="right">
    {#if accel}
      <span
        class="accel-badge"
        class:gpu={isGpu}
        title={tooltip}
      >
        <span class="accel-dot"></span>
        {accel.backend}
      </span>
    {/if}
    <a
      class="model-info"
      href="https://huggingface.co/{MODEL_REPO}"
      target="_blank"
      rel="noreferrer"
      title="Otwórz stronę modelu na HuggingFace"
    >
      <span class="model-name">{MODEL_REPO}</span>
      <span class="model-quant">{MODEL_QUANTIZATION}</span>
    </a>
  </div>
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

  .right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .accel-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    padding: 3px 8px;
    border-radius: 5px;
    border: 1px solid var(--border);
    background: var(--bg-subtle);
    -webkit-app-region: no-drag;
    app-region: no-drag;
    cursor: default;
    user-select: none;
    white-space: nowrap;
  }

  .accel-badge.gpu {
    color: var(--accent-text);
    background: var(--accent-soft-bg);
    border-color: var(--accent-soft-border);
  }

  .accel-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
  }

  .accel-badge.gpu .accel-dot {
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 25%, transparent);
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
