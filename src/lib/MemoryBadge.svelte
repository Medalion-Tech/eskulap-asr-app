<script lang="ts">
  import { invoke, Channel } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";

  interface MemorySnapshot {
    app_rss_mb: number;
    system_total_mb: number;
    system_used_mb: number;
    gpu_used_mb: number | null;
    gpu_total_mb: number | null;
    asr_mb: number | null;
    llm_mb: number | null;
    backend: string;
    unified_memory: boolean;
  }

  let snap: MemorySnapshot | null = $state(null);
  let open = $state(false);
  let channel: Channel<MemorySnapshot> | null = null;

  onMount(async () => {
    try {
      channel = new Channel<MemorySnapshot>();
      channel.onmessage = (msg) => {
        snap = msg;
      };
      await invoke("subscribe_memory_stats", { onUpdate: channel });
    } catch (e) {
      console.error("Failed to subscribe to memory stats:", e);
    }
  });

  onDestroy(() => {
    channel = null;
  });

  function fmtMb(mb: number): string {
    if (mb >= 1024) {
      return (mb / 1024).toFixed(2) + " GB";
    }
    return mb.toFixed(0) + " MB";
  }

  function fmtOpt(mb: number | null): string {
    return mb == null ? "—" : fmtMb(mb);
  }

  const compact = $derived(snap ? fmtMb(snap.app_rss_mb) : "—");

  const ramPct = $derived(
    snap && snap.system_total_mb > 0
      ? Math.round((snap.system_used_mb / snap.system_total_mb) * 100)
      : 0
  );

  const gpuPct = $derived(
    snap && snap.gpu_used_mb != null && snap.gpu_total_mb && snap.gpu_total_mb > 0
      ? Math.round((snap.gpu_used_mb / snap.gpu_total_mb) * 100)
      : 0
  );

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") open = false;
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="mem-wrap">
  <button
    class="mem-badge"
    onclick={() => (open = !open)}
    title="Zużycie pamięci — kliknij, aby zobaczyć rozbicie"
    aria-label="Zużycie pamięci"
    aria-expanded={open}
  >
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="7" width="18" height="10" rx="1.5" />
      <path d="M7 7v10" />
      <path d="M11 7v10" />
      <path d="M15 7v10" />
      <path d="M19 7v10" />
    </svg>
    <span class="mem-val">{compact}</span>
  </button>

  {#if open && snap}
    <div class="mem-popover" role="dialog">
      <div class="row header">
        <span>Pamięć aplikacji</span>
        <button class="close-btn" onclick={() => (open = false)} aria-label="Zamknij">×</button>
      </div>
      <div class="row big">
        <span class="label">RAM procesu (RSS)</span>
        <span class="value">{fmtMb(snap.app_rss_mb)}</span>
      </div>
      <div class="row">
        <span class="label">ASR (Whisper)</span>
        <span class="value">{fmtOpt(snap.asr_mb)}</span>
      </div>
      <div class="row">
        <span class="label">LLM (llama.cpp)</span>
        <span class="value">{fmtOpt(snap.llm_mb)}</span>
      </div>

      <div class="divider"></div>

      <div class="row sub">
        <span class="label">Backend</span>
        <span class="value">{snap.backend}{snap.unified_memory ? " · unified" : ""}</span>
      </div>

      {#if snap.unified_memory}
        <div class="note">
          Apple Silicon — GPU dzieli pamięć z CPU (unified memory),
          więc RAM procesu zawiera już alokacje akceleratora.
        </div>
      {:else if snap.gpu_used_mb != null}
        <div class="row">
          <span class="label">GPU (cały system)</span>
          <span class="value">
            {fmtMb(snap.gpu_used_mb)}{snap.gpu_total_mb ? ` / ${fmtMb(snap.gpu_total_mb)}` : ""}
            {snap.gpu_total_mb ? ` · ${gpuPct}%` : ""}
          </span>
        </div>
        <div class="bar"><div class="fill gpu" style="width:{gpuPct}%"></div></div>
      {/if}

      <div class="divider"></div>

      <div class="row sub">
        <span class="label">System RAM</span>
        <span class="value">{fmtMb(snap.system_used_mb)} / {fmtMb(snap.system_total_mb)} · {ramPct}%</span>
      </div>
      <div class="bar"><div class="fill ram" style="width:{ramPct}%"></div></div>

      <div class="footer">Aktualizacja co 1 s. Wartości ASR/LLM to delta RSS przy ładowaniu modelu.</div>
    </div>
  {/if}
</div>

<style>
  .mem-wrap {
    position: relative;
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .mem-badge {
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
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    transition: background var(--duration-fast) var(--easing),
      color var(--duration-fast) var(--easing);
  }

  .mem-badge:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .mem-val {
    font-variant-numeric: tabular-nums;
  }

  .mem-popover {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 1000;
    min-width: 260px;
    padding: 10px 12px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    font-size: 12px;
    color: var(--text);
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 3px 0;
    font-variant-numeric: tabular-nums;
  }

  .row.header {
    font-weight: 600;
    margin-bottom: 4px;
  }

  .row.big .value {
    font-weight: 600;
  }

  .row.sub {
    color: var(--text-muted);
    font-size: 11px;
  }

  .label {
    color: var(--text-muted);
  }

  .row.big .label {
    color: var(--text);
  }

  .value {
    color: var(--text);
  }

  .divider {
    height: 1px;
    background: var(--border);
    margin: 6px 0;
  }

  .bar {
    height: 4px;
    background: var(--bg-subtle);
    border-radius: 2px;
    overflow: hidden;
    margin: 2px 0 4px;
  }

  .fill {
    height: 100%;
    background: var(--text-muted);
    transition: width 300ms var(--easing);
  }

  .fill.ram {
    background: var(--accent, #4a90e2);
  }

  .fill.gpu {
    background: var(--accent, #4a90e2);
  }

  .note {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
    padding: 4px 0 2px;
  }

  .footer {
    font-size: 10px;
    color: var(--text-muted);
    margin-top: 6px;
    line-height: 1.4;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    padding: 0 4px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
</style>
