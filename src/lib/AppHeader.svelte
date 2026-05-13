<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { screen } from "./stores";
  import MemoryBadge from "./MemoryBadge.svelte";

  interface AcceleratorInfo {
    backend: string;
    device: string;
    platform: string;
    arch: string;
    threads: number;
    cpu_model: string;
  }

  let accel: AcceleratorInfo | null = $state(null);
  let version: string = $state("");

  onMount(async () => {
    try {
      accel = await invoke<AcceleratorInfo>("get_accelerator_info");
    } catch (e) {
      console.error("Failed to get accelerator info:", e);
    }
    try {
      version = await invoke<string>("get_app_version");
    } catch (e) {
      console.error("Failed to get app version:", e);
    }
  });

  const isGpu = $derived(
    accel?.backend === "Metal" ||
      accel?.backend === "Vulkan" ||
      accel?.backend === "CUDA"
  );
  const tooltip = $derived(
    accel
      ? `${accel.platform} ${accel.arch}` +
        (accel.device ? ` · ${accel.device}` : "") +
        (accel.backend === "CPU" ? ` · ${accel.threads} wątków` : "") +
        (accel.backend !== "CPU" && accel.cpu_model ? ` · CPU: ${accel.cpu_model}` : "")
      : ""
  );
</script>

<header class="app-header">
  <div class="title-group">
    <span class="app-name">Eskulap ASR</span>
    {#if version}
      <span class="version-badge" title="Wersja aplikacji">v{version}</span>
    {/if}
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
    <MemoryBadge />
    <button
      class="icon-btn"
      onclick={() => ($screen = "templates")}
      title="Szablony"
      aria-label="Szablony"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
        <path d="M6 2h9l5 5v15H6z" />
        <path d="M14 2v6h6" />
        <path d="M9 13h6" />
        <path d="M9 17h6" />
      </svg>
    </button>
    <button
      class="icon-btn"
      onclick={() => ($screen = "settings")}
      title="Ustawienia"
      aria-label="Ustawienia"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
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

  .version-badge {
    font-size: 10px;
    font-weight: 500;
    color: var(--text-muted);
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--bg-subtle);
    border: 1px solid var(--border);
    user-select: none;
    font-variant-numeric: tabular-nums;
    -webkit-app-region: no-drag;
    app-region: no-drag;
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

  .icon-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    -webkit-app-region: no-drag;
    app-region: no-drag;
    transition: background var(--duration-fast) var(--easing),
      color var(--duration-fast) var(--easing);
  }

  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
</style>
