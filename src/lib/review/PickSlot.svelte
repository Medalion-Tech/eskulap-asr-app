<script lang="ts">
  import type { FilledValue, Slot } from "../ast-types";

  interface Props {
    slot: Slot & { kind: "pick" };
    value: FilledValue | undefined;
    edited: boolean;
    onchange: (v: FilledValue) => void;
  }

  let { slot, value, edited, onchange }: Props = $props();

  const unfilled = $derived(
    !value || value.kind === "unfilled" || (value.kind === "pick" && value.code === "X"),
  );

  const currentLabel = $derived.by(() => {
    if (!value || value.kind !== "pick") return "[nie wspomniano]";
    if (value.code === "other") return value.custom_text ?? "[opis własny]";
    if (value.code === "X") return "[nie wspomniano]";
    const opt = slot.options.find((o) => o.code === value.code);
    return opt ? opt.text : "[?]";
  });

  let open = $state(false);
  let otherEditing = $state(false);
  let otherDraft = $state("");

  function choose(code: string) {
    if (code === "other") {
      otherDraft = value && value.kind === "pick" && value.code === "other" ? value.custom_text ?? "" : "";
      otherEditing = true;
      return;
    }
    open = false;
    if (code === "X") {
      onchange({ kind: "unfilled" });
    } else {
      onchange({ kind: "pick", code, custom_text: null });
    }
  }

  function commitOther() {
    const trimmed = otherDraft.trim();
    otherEditing = false;
    open = false;
    if (!trimmed) {
      onchange({ kind: "unfilled" });
    } else {
      onchange({ kind: "pick", code: "other", custom_text: trimmed });
    }
  }
</script>

<span class="pick-wrap">
  <button
    type="button"
    class="pick-chip"
    class:filled={!unfilled && !edited}
    class:unfilled
    class:edited
    onclick={() => (open = !open)}
    title={slot.hint ?? slot.name}
  >
    {currentLabel}
    <span class="caret">▾</span>
  </button>

  {#if open}
    <div class="menu" role="menu">
      {#each slot.options as opt}
        <button type="button" class="menu-item" onclick={() => choose(opt.code)}>
          <span class="code">{opt.code}</span>
          <span>{opt.text}</span>
        </button>
      {/each}
      {#if slot.allow_other}
        <button type="button" class="menu-item" onclick={() => choose("other")}>
          <span class="code">…</span>
          <span>Inne (opis własny)</span>
        </button>
      {/if}
    </div>
  {/if}

  {#if otherEditing}
    <span class="other-editor">
      <input
        bind:value={otherDraft}
        onkeydown={(e) => {
          if (e.key === "Enter") commitOther();
          else if (e.key === "Escape") (otherEditing = false);
        }}
        onblur={commitOther}
        placeholder="Opis własny"
      />
    </span>
  {/if}
</span>

<style>
  .pick-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .pick-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: 1px solid transparent;
    padding: 1px 6px;
    border-radius: 3px;
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  .pick-chip:hover {
    border-color: var(--border);
  }
  .pick-chip.filled {
    background: var(--slot-filled-bg);
  }
  .pick-chip.unfilled {
    background: var(--slot-unfilled-bg);
    color: color-mix(in srgb, var(--text) 70%, #92400e);
  }
  .pick-chip.edited {
    background: var(--slot-edited-bg);
  }
  .caret {
    font-size: 9px;
    color: var(--text-muted);
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    padding: 4px;
    min-width: 180px;
    z-index: 10;
    display: flex;
    flex-direction: column;
  }
  .menu-item {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 8px;
    background: transparent;
    border: none;
    text-align: left;
    font: inherit;
    color: inherit;
    border-radius: 4px;
    cursor: pointer;
  }
  .menu-item:hover {
    background: var(--bg-subtle);
  }
  .code {
    display: inline-block;
    min-width: 14px;
    font-weight: 600;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .other-editor {
    margin-left: 4px;
  }
  .other-editor input {
    padding: 2px 6px;
    border: 1px solid var(--accent);
    border-radius: 3px;
    font: inherit;
    background: var(--surface);
    color: inherit;
  }
</style>
