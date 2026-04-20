<script lang="ts">
  import type { FilledValue, Slot } from "../ast-types";

  interface Props {
    slot: Slot & { kind: "field" };
    value: FilledValue | undefined;
    edited: boolean;
    onchange: (v: FilledValue) => void;
  }

  let { slot, value, edited, onchange }: Props = $props();

  const unfilled = $derived(!value || value.kind === "unfilled");
  const initialText = $derived.by(() => {
    if (!value || value.kind !== "text") return "";
    return value.text;
  });

  let editing = $state(false);
  let draft = $state("");
  let inputEl: HTMLInputElement | null = $state(null);

  function begin() {
    draft = initialText;
    editing = true;
    queueMicrotask(() => inputEl?.focus());
  }

  function commit() {
    editing = false;
    const trimmed = draft.trim();
    if (!trimmed) {
      onchange({ kind: "unfilled" });
    } else {
      onchange({ kind: "text", text: trimmed });
    }
  }

  function cancel() {
    editing = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commit();
    } else if (e.key === "Escape") {
      cancel();
    }
  }
</script>

{#if editing}
  <input
    bind:this={inputEl}
    class="slot-input"
    bind:value={draft}
    onblur={commit}
    onkeydown={onkey}
    placeholder={slot.hint ?? ""}
  />
{:else}
  <button
    type="button"
    class="slot-chip"
    class:filled={!unfilled && !edited}
    class:unfilled
    class:edited
    onclick={begin}
    title={slot.hint ?? slot.name}
  >
    {#if unfilled}
      [nie wspomniano]
    {:else}
      {initialText}
    {/if}
  </button>
{/if}

<style>
  .slot-chip {
    display: inline;
    background: transparent;
    border: none;
    padding: 1px 4px;
    border-radius: 3px;
    font: inherit;
    color: inherit;
    cursor: text;
    border-bottom: 1px dashed transparent;
    text-align: left;
  }
  .slot-chip:hover {
    border-bottom-color: var(--border);
  }
  .slot-chip.filled {
    background: var(--slot-filled-bg);
  }
  .slot-chip.unfilled {
    background: var(--slot-unfilled-bg);
    color: color-mix(in srgb, var(--text) 70%, #92400e);
  }
  .slot-chip.edited {
    background: var(--slot-edited-bg);
  }
  .slot-input {
    display: inline-block;
    padding: 1px 4px;
    font: inherit;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--accent);
    border-radius: 3px;
    min-width: 120px;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .slot-input:focus {
    outline: none;
  }
</style>
