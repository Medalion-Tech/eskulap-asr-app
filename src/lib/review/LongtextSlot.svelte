<script lang="ts">
  import type { FilledValue, Slot } from "../ast-types";

  interface Props {
    slot: Slot & { kind: "longtext" };
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
  let ta: HTMLTextAreaElement | null = $state(null);

  function begin() {
    draft = initialText;
    editing = true;
    queueMicrotask(() => {
      ta?.focus();
      resize();
    });
  }

  function resize() {
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = ta.scrollHeight + "px";
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

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      editing = false;
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      commit();
    }
  }
</script>

{#if editing}
  <textarea
    bind:this={ta}
    class="slot-textarea"
    bind:value={draft}
    oninput={resize}
    onblur={commit}
    onkeydown={onkey}
    placeholder={slot.hint ?? ""}
  ></textarea>
{:else}
  <button
    type="button"
    class="slot-block"
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
  .slot-block {
    display: block;
    width: 100%;
    padding: 6px 8px;
    border-radius: 4px;
    font: inherit;
    color: inherit;
    background: transparent;
    border: 1px solid transparent;
    text-align: left;
    white-space: pre-wrap;
    cursor: text;
  }
  .slot-block:hover {
    border-color: var(--border);
  }
  .slot-block.filled {
    background: var(--slot-filled-bg);
    border-color: var(--slot-filled-border);
  }
  .slot-block.unfilled {
    background: var(--slot-unfilled-bg);
    border-color: var(--slot-unfilled-border);
    color: color-mix(in srgb, var(--text) 70%, #92400e);
  }
  .slot-block.edited {
    background: var(--slot-edited-bg);
    border-color: var(--slot-edited-border);
  }
  .slot-textarea {
    display: block;
    width: 100%;
    padding: 6px 8px;
    font: inherit;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--accent);
    border-radius: 4px;
    resize: none;
    min-height: 40px;
    line-height: 1.55;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }
  .slot-textarea:focus {
    outline: none;
  }
</style>
