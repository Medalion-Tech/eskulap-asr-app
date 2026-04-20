<script lang="ts">
  import type { FilledValue, Slot } from "../ast-types";

  interface Props {
    slot: Slot & { kind: "list" };
    value: FilledValue | undefined;
    edited: boolean;
    onchange: (v: FilledValue) => void;
  }

  let { slot, value, edited, onchange }: Props = $props();

  const items = $derived.by<string[]>(() => {
    if (value && value.kind === "list") return value.items;
    return [];
  });
  const unfilled = $derived(items.length === 0);

  function commit(next: string[]) {
    const cleaned = next.map((s) => s.trim()).filter((s) => s.length > 0);
    if (cleaned.length === 0) {
      onchange({ kind: "unfilled" });
    } else {
      onchange({ kind: "list", items: cleaned });
    }
  }

  function updateItem(idx: number, text: string) {
    const next = [...items];
    next[idx] = text;
    commit(next);
  }
  function removeItem(idx: number) {
    const next = items.filter((_, i) => i !== idx);
    commit(next);
  }
  function addItem() {
    commit([...items, "Nowy punkt"]);
  }
</script>

<div class="list-wrap">
  {#if unfilled}
    <button type="button" class="add-first" onclick={addItem}>
      [nie wspomniano] — dodaj punkt
    </button>
  {:else}
    <ol class:numbered={slot.numbered} class:bullets={!slot.numbered}>
      {#each items as item, i (i)}
        <li>
          <input
            class="item-input"
            class:filled={!edited}
            class:edited
            value={item}
            onblur={(e) => updateItem(i, (e.currentTarget as HTMLInputElement).value)}
            onkeydown={(e) => {
              if (e.key === "Enter") {
                (e.currentTarget as HTMLInputElement).blur();
                e.preventDefault();
              }
            }}
          />
          <button type="button" class="remove" onclick={() => removeItem(i)} aria-label="Usuń punkt">×</button>
        </li>
      {/each}
    </ol>
    <button type="button" class="add-btn" onclick={addItem}>+ Dodaj punkt</button>
  {/if}
</div>

<style>
  .list-wrap {
    padding: 0;
  }
  ol {
    margin: 0;
    padding-left: 22px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  ol.bullets {
    list-style: disc;
  }
  ol.numbered {
    list-style: decimal;
  }
  li {
    display: list-item;
  }
  li::marker {
    color: var(--text-muted);
  }
  li > :global(*) {
    vertical-align: middle;
  }
  .item-input {
    display: inline-block;
    width: calc(100% - 30px);
    padding: 1px 6px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
    font: inherit;
    color: inherit;
    vertical-align: middle;
  }
  .item-input.filled {
    background: var(--slot-filled-bg);
  }
  .item-input.edited {
    background: var(--slot-edited-bg);
  }
  .item-input:hover {
    border-bottom: 1px dashed var(--border);
  }
  .item-input:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--surface);
  }
  .remove {
    display: inline-block;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 14px;
    padding: 0 4px;
    margin-left: 2px;
    vertical-align: middle;
    line-height: 1;
  }
  .remove:hover {
    color: var(--danger);
  }
  .add-btn,
  .add-first {
    margin-top: 6px;
    background: transparent;
    border: 1px dashed var(--border);
    padding: 3px 10px;
    font-size: 12px;
    color: var(--text-muted);
    border-radius: 3px;
    cursor: pointer;
  }
  .add-btn:hover,
  .add-first:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .add-first {
    display: block;
    width: 100%;
    text-align: left;
  }
</style>
