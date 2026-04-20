<script lang="ts">
  import { templates, selectedTemplateId } from "./stores";

  let open = $state(false);
  let wrapper: HTMLDivElement;

  const selected = $derived(
    $selectedTemplateId
      ? $templates.find((t) => t.id === $selectedTemplateId) ?? null
      : null
  );

  function pick(id: string | null) {
    $selectedTemplateId = id;
    open = false;
  }

  function onDocClick(e: MouseEvent) {
    if (wrapper && !wrapper.contains(e.target as Node)) {
      open = false;
    }
  }

  $effect(() => {
    if (open) {
      document.addEventListener("mousedown", onDocClick);
      return () => document.removeEventListener("mousedown", onDocClick);
    }
  });
</script>

<div class="picker" bind:this={wrapper}>
  <button
    class="trigger"
    class:active={open}
    onclick={() => (open = !open)}
    aria-haspopup="listbox"
    aria-expanded={open}
  >
    <span class="trigger-label">Szablon</span>
    <span class="trigger-value">{selected ? selected.name : "Bez szablonu"}</span>
    <svg class="chev" class:open width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="2 4 5 7 8 4" />
    </svg>
  </button>

  {#if open}
    <div class="menu" role="listbox">
      <button
        class="item"
        class:selected={$selectedTemplateId === null}
        onclick={() => pick(null)}
      >
        <span class="item-name">Bez szablonu</span>
        <span class="item-desc">Surowa transkrypcja bez LLM</span>
      </button>
      {#each $templates as t (t.id)}
        <button
          class="item"
          class:selected={$selectedTemplateId === t.id}
          onclick={() => pick(t.id)}
        >
          <span class="item-name">
            {t.name}
            {#if t.is_builtin}
              <span class="badge">wbudowany</span>
            {/if}
          </span>
          {#if t.description}
            <span class="item-desc">{t.description}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
    width: 100%;
    max-width: 420px;
    margin: 12px auto 0;
    padding: 0 20px;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 13px;
    transition: border-color var(--duration-fast) var(--easing),
      background var(--duration-fast) var(--easing);
  }

  .trigger:hover,
  .trigger.active {
    border-color: var(--border-strong);
    background: var(--bg-hover);
  }

  .trigger-label {
    color: var(--text-muted);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 600;
  }

  .trigger-value {
    flex: 1;
    text-align: left;
    color: var(--text);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chev {
    color: var(--text-muted);
    transition: transform var(--duration-fast) var(--easing);
  }

  .chev.open {
    transform: rotate(180deg);
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 20px;
    right: 20px;
    max-height: 320px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    z-index: 50;
    padding: 4px;
  }

  .item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm, 6px);
    text-align: left;
    cursor: pointer;
    transition: background var(--duration-fast) var(--easing);
  }

  .item:hover {
    background: var(--bg-hover);
  }

  .item.selected {
    background: var(--accent-soft-bg);
  }

  .item-name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .item-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .badge {
    font-size: 9px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-active);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
</style>
