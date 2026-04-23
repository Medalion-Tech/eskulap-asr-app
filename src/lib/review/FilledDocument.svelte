<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type {
    FilledTemplate,
    FilledValue,
    Inline,
    Node,
    SlotId,
    TemplateAst,
  } from "../ast-types";
  import type { Note } from "../stores";
  import { notes } from "../stores";
  import FieldSlot from "./FieldSlot.svelte";
  import LongtextSlot from "./LongtextSlot.svelte";
  import PickSlot from "./PickSlot.svelte";
  import ListSlot from "./ListSlot.svelte";

  interface Props {
    noteId: string;
    ast: TemplateAst;
    filled: FilledTemplate;
  }

  let { noteId, ast, filled: propFilled }: Props = $props();
  // `propFilled` is a Svelte reactive proxy — structuredClone throws on it.
  // JSON roundtrip unwraps cleanly (our values are plain serde-style data).
  let filled = $state<FilledTemplate>(JSON.parse(JSON.stringify(propFilled)) as FilledTemplate);

  async function updateSlot(slotId: SlotId, value: FilledValue) {
    try {
      const updated = await invoke<Note>("update_filled_value", {
        noteId,
        slotId,
        value,
      });
      if (updated.filled) {
        filled = updated.filled;
      }
      $notes = $notes.map((n) => (n.id === updated.id ? { ...updated, selected: n.selected } : n));
    } catch (e) {
      console.error("update_filled_value failed", e);
    }
  }

  function slotOfInline(inline: Inline) {
    if (inline.kind !== "slot") return null;
    return ast.slots[inline.id] ?? null;
  }
</script>

<article class="doc">
  {#each ast.nodes as node, nodeIdx (nodeIdx)}
    {#if node.kind === "heading"}
      {#if node.level === 1}
        <h1 class="doc-h1">
          {#each node.inlines as inl}
            {#if inl.kind === "text"}{inl.text}{/if}
          {/each}
        </h1>
      {:else}
        <h2 class="doc-h2">
          {#each node.inlines as inl}
            {#if inl.kind === "text"}{inl.text}{/if}
          {/each}
        </h2>
      {/if}
    {:else if node.kind === "paragraph"}
      <p class="doc-p">
        {#each node.inlines as inl, inlIdx (inlIdx)}
          {#if inl.kind === "text"}
            {inl.text}
          {:else}
            {@const slot = slotOfInline(inl)}
            {#if slot}
              {#if slot.kind === "field"}
                <FieldSlot
                  {slot}
                  value={filled.values[slot.id]}
                  edited={filled.user_edited.includes(slot.id)}
                  onchange={(v) => updateSlot(slot.id, v)}
                />
              {:else if slot.kind === "pick"}
                <PickSlot
                  {slot}
                  value={filled.values[slot.id]}
                  edited={filled.user_edited.includes(slot.id)}
                  onchange={(v) => updateSlot(slot.id, v)}
                />
              {:else if slot.kind === "longtext"}
                <LongtextSlot
                  {slot}
                  value={filled.values[slot.id]}
                  edited={filled.user_edited.includes(slot.id)}
                  onchange={(v) => updateSlot(slot.id, v)}
                />
              {:else if slot.kind === "list"}
                <ListSlot
                  {slot}
                  value={filled.values[slot.id]}
                  edited={filled.user_edited.includes(slot.id)}
                  onchange={(v) => updateSlot(slot.id, v)}
                />
              {/if}
            {/if}
          {/if}
        {/each}
      </p>
    {:else if node.kind === "slot_block"}
      {@const slot = ast.slots[node.id]}
      {#if slot}
        <div class="doc-block">
          {#if slot.kind === "longtext"}
            <LongtextSlot
              {slot}
              value={filled.values[slot.id]}
              edited={filled.user_edited.includes(slot.id)}
              onchange={(v) => updateSlot(slot.id, v)}
            />
          {:else if slot.kind === "list"}
            <ListSlot
              {slot}
              value={filled.values[slot.id]}
              edited={filled.user_edited.includes(slot.id)}
              onchange={(v) => updateSlot(slot.id, v)}
            />
          {:else if slot.kind === "pick"}
            <PickSlot
              {slot}
              value={filled.values[slot.id]}
              edited={filled.user_edited.includes(slot.id)}
              onchange={(v) => updateSlot(slot.id, v)}
            />
          {:else if slot.kind === "field"}
            <FieldSlot
              {slot}
              value={filled.values[slot.id]}
              edited={filled.user_edited.includes(slot.id)}
              onchange={(v) => updateSlot(slot.id, v)}
            />
          {/if}
        </div>
      {/if}
    {/if}
  {/each}
</article>

<style>
  .doc {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--text);

    --slot-filled-bg: color-mix(in srgb, #22c55e 12%, transparent);
    --slot-filled-border: color-mix(in srgb, #22c55e 40%, transparent);
    --slot-unfilled-bg: color-mix(in srgb, #eab308 20%, transparent);
    --slot-unfilled-border: color-mix(in srgb, #eab308 55%, transparent);
    --slot-edited-bg: color-mix(in srgb, #3b82f6 14%, transparent);
    --slot-edited-border: color-mix(in srgb, #3b82f6 45%, transparent);
  }

  .doc-h1 {
    font-size: 15px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.02em;
    margin: 14px 0 4px;
    color: var(--text);
  }
  .doc-h1:first-child {
    margin-top: 0;
  }

  .doc-h2 {
    font-size: 13px;
    font-weight: 600;
    margin: 12px 0 2px;
    color: var(--text);
  }

  .doc-p {
    margin: 0;
  }

  .doc-block {
    margin: 2px 0;
  }
</style>
