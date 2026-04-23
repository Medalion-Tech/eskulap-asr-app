<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import { templates } from "../stores";
  import type { Template } from "../stores";
  import type { TemplateAst, Slot, PickOption } from "../ast-types";
  import { SlotNode } from "./slot-nodes";
  import { astToDoc, docToAst } from "./serde";

  interface Props {
    template: Template | null;
    onclose: () => void;
  }

  let { template, onclose }: Props = $props();

  const isNew = template === null || template.id === "";
  const canEdit = !template?.is_builtin;

  let editorEl: HTMLDivElement | null = $state(null);
  let editor: Editor | null = null;

  let name = $state(template?.name ?? "");
  let description = $state(template?.description ?? "");
  let exampleInput = $state(template?.example_input ?? "");
  let saving = $state(false);
  let error = $state("");

  // Slash menu
  let menuOpen = $state(false);
  let menuIndex = $state(0);
  let menuX = $state(0);
  let menuY = $state(0);
  const menuItems = [
    { kind: "field", label: "Pole (krótki tekst)" },
    { kind: "longtext", label: "Długi tekst (akapit)" },
    { kind: "pick", label: "Lista wyboru" },
    { kind: "list", label: "Lista punktowa" },
  ] as const;

  // Edit-slot popover
  let editingSlotPos: number | null = $state(null);
  let editingAttrs: Record<string, any> = $state({});
  let editingOptions: PickOption[] = $state([]);

  function uuid(): string {
    if (typeof crypto !== "undefined" && crypto.randomUUID) {
      return crypto.randomUUID();
    }
    return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0;
      return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
    });
  }

  onMount(() => {
    if (!editorEl) return;
    const doc = template
      ? astToDoc(template.ast)
      : { type: "doc", content: [{ type: "paragraph" }] };

    editor = new Editor({
      element: editorEl,
      extensions: [
        StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
        SlotNode,
      ],
      editable: canEdit,
      content: doc,
      onTransaction: ({ editor: ed }) => {
        const state = ed.state;
        const from = state.selection.$from;
        const text = from.parent.textBetween(0, from.parentOffset, "\n", "\n");
        if (text.endsWith("/")) {
          const coords = ed.view.coordsAtPos(from.pos);
          const rect = editorEl!.getBoundingClientRect();
          menuX = coords.left - rect.left;
          menuY = coords.bottom - rect.top + 4;
          menuOpen = true;
          menuIndex = 0;
        } else if (menuOpen) {
          menuOpen = false;
        }
      },
    });

    // Click on slot → open attribute editor
    editor.view.dom.addEventListener("click", onEditorClick);
  });

  onDestroy(() => {
    editor?.view.dom.removeEventListener("click", onEditorClick);
    editor?.destroy();
  });

  function onEditorClick(e: MouseEvent) {
    if (!editor) return;
    const target = e.target as HTMLElement;
    const chip = target.closest("[data-slot]");
    if (!chip) return;
    // Find position of this slot node in the doc.
    let pos: number | null = null;
    editor.state.doc.descendants((node, p) => {
      if (node.type.name === "slot") {
        const id = node.attrs.slotId;
        if (chip.getAttribute("slotid") === id || chip.getAttribute("data-slotid") === id) {
          pos = p;
          return false;
        }
      }
      return true;
    });
    // Fallback: first slot at the current selection.
    if (pos === null) {
      const sel = editor.state.selection;
      editor.state.doc.descendants((node, p) => {
        if (node.type.name === "slot" && p >= sel.from - 1 && p <= sel.to + 1) {
          pos = p;
          return false;
        }
        return true;
      });
    }
    if (pos !== null) openSlotEditor(pos);
  }

  function openSlotEditor(pos: number) {
    if (!editor || !canEdit) return;
    const node = editor.state.doc.nodeAt(pos);
    if (!node || node.type.name !== "slot") return;
    editingSlotPos = pos;
    editingAttrs = { ...node.attrs };
    try {
      editingOptions = JSON.parse(editingAttrs.options || "[]");
    } catch {
      editingOptions = [];
    }
    if (editingAttrs.slotKind === "pick" && editingOptions.length === 0) {
      editingOptions = [
        { code: "A", text: "" },
        { code: "B", text: "" },
        { code: "X", text: "nieokreślone" },
      ];
    }
  }

  function commitSlotEdit() {
    if (!editor || editingSlotPos === null) return;
    const attrs = { ...editingAttrs };
    if (attrs.slotKind === "pick") {
      attrs.options = JSON.stringify(editingOptions.filter((o) => o.code && o.text));
    }
    editor
      .chain()
      .focus()
      .command(({ tr, state }) => {
        const node = state.doc.nodeAt(editingSlotPos!);
        if (!node) return false;
        tr.setNodeMarkup(editingSlotPos!, undefined, attrs);
        return true;
      })
      .run();
    editingSlotPos = null;
  }

  function cancelSlotEdit() {
    editingSlotPos = null;
  }

  function deleteSlot() {
    if (!editor || editingSlotPos === null) return;
    const pos = editingSlotPos;
    editor
      .chain()
      .focus()
      .command(({ tr }) => {
        tr.delete(pos, pos + 1);
        return true;
      })
      .run();
    editingSlotPos = null;
  }

  function insertSlot(kind: (typeof menuItems)[number]["kind"]) {
    if (!editor) return;
    // Delete the trigger "/" char behind cursor.
    editor
      .chain()
      .focus()
      .command(({ tr, state }) => {
        const from = state.selection.from;
        if (from > 0 && state.doc.textBetween(from - 1, from) === "/") {
          tr.delete(from - 1, from);
        }
        return true;
      })
      .run();

    const attrs: Record<string, any> = {
      slotId: uuid(),
      slotKind: kind,
      name: `nowy_${kind}`,
      hint: null,
    };
    if (kind === "field") attrs.default = null;
    if (kind === "pick") {
      attrs.options = JSON.stringify([
        { code: "A", text: "" },
        { code: "B", text: "" },
        { code: "X", text: "nieokreślone" },
      ]);
      attrs.allowOther = "true";
    }
    if (kind === "list") attrs.numbered = "false";

    editor
      .chain()
      .focus()
      .insertContent({ type: "slot", attrs })
      .run();
    menuOpen = false;

    // Find inserted pos and open editor immediately.
    let insertedPos: number | null = null;
    editor.state.doc.descendants((node, p) => {
      if (node.type.name === "slot" && node.attrs.slotId === attrs.slotId) {
        insertedPos = p;
        return false;
      }
      return true;
    });
    if (insertedPos !== null) {
      setTimeout(() => openSlotEditor(insertedPos!), 30);
    }
  }

  function onEditorKeydown(e: KeyboardEvent) {
    if (!menuOpen) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      menuIndex = (menuIndex + 1) % menuItems.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      menuIndex = (menuIndex - 1 + menuItems.length) % menuItems.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      insertSlot(menuItems[menuIndex].kind);
    } else if (e.key === "Escape") {
      menuOpen = false;
    }
  }

  function addPickOption() {
    const nextCode = String.fromCharCode(65 + editingOptions.filter((o) => o.code !== "X").length);
    editingOptions = [
      ...editingOptions.filter((o) => o.code !== "X"),
      { code: nextCode, text: "" },
      ...editingOptions.filter((o) => o.code === "X"),
    ];
  }

  function removePickOption(i: number) {
    editingOptions = editingOptions.filter((_, idx) => idx !== i);
  }

  async function save() {
    if (!editor) return;
    error = "";
    if (!name.trim()) {
      error = "Nazwa jest wymagana.";
      return;
    }
    saving = true;
    try {
      const ast: TemplateAst = docToAst(editor.getJSON());
      const payload: Template = {
        id: template?.id ?? "",
        name: name.trim(),
        description: description.trim(),
        ast,
        example_input: exampleInput.trim() || null,
        example_filled: template?.example_filled ?? null,
        is_builtin: false,
        created_at: template?.created_at ?? "",
        updated_at: template?.updated_at ?? "",
        ast_version: 1,
      };
      if (isNew) {
        const created = await invoke<Template>("add_template", { template: payload });
        $templates = [...$templates, created];
      } else {
        const updated = await invoke<Template>("update_template", {
          id: template!.id,
          template: payload,
        });
        $templates = $templates.map((t) => (t.id === updated.id ? updated : t));
      }
      onclose();
    } catch (e: any) {
      error = e?.toString() ?? "Nie udało się zapisać";
    } finally {
      saving = false;
    }
  }
</script>

<section class="editor">
  <header class="editor-header">
    <button class="btn btn-ghost" onclick={onclose}>
      <span class="arrow">←</span> Powrót
    </button>
    <h2>{isNew ? "Nowy szablon" : canEdit ? "Edycja szablonu" : "Podgląd (wbudowany)"}</h2>
    {#if canEdit}
      <button class="btn btn-solid" onclick={save} disabled={saving}>
        {saving ? "Zapisywanie…" : "Zapisz"}
      </button>
    {:else}
      <span class="spacer"></span>
    {/if}
  </header>

  <div class="form">
    <label class="field">
      <span class="label">Nazwa</span>
      <input type="text" bind:value={name} disabled={!canEdit} />
    </label>
    <label class="field">
      <span class="label">Opis</span>
      <input type="text" bind:value={description} disabled={!canEdit} />
    </label>

    <div class="field">
      <span class="label">Szablon</span>
      <span class="hint">Napisz strukturę notatki. Wpisz „/" aby wstawić slot.</span>
      <div class="tiptap-wrap" role="region" aria-label="Edytor szablonu">
        <div class="tiptap" bind:this={editorEl} onkeydown={onEditorKeydown}></div>

        {#if menuOpen}
          <div class="slash-menu" style="left: {menuX}px; top: {menuY}px">
            {#each menuItems as item, i}
              <button
                type="button"
                class="slash-item"
                class:active={i === menuIndex}
                onmouseenter={() => (menuIndex = i)}
                onclick={() => insertSlot(item.kind)}
              >
                {item.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <label class="field">
      <span class="label">Przykład dyktowania (opcjonalnie)</span>
      <textarea bind:value={exampleInput} rows="4" disabled={!canEdit}></textarea>
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  {#if editingSlotPos !== null}
    <div
      class="slot-editor-backdrop"
      role="button"
      tabindex="0"
      onclick={cancelSlotEdit}
      onkeydown={(e) => {
        if (e.key === "Escape") cancelSlotEdit();
      }}
    ></div>
    <div class="slot-editor" role="dialog" aria-label="Edycja slotu">
      <h3>{editingAttrs.slotKind === "field" ? "Pole" : editingAttrs.slotKind === "longtext" ? "Długi tekst" : editingAttrs.slotKind === "pick" ? "Lista wyboru" : "Lista punktowa"}</h3>
      <label class="se-field">
        <span>Nazwa (snake_case)</span>
        <input bind:value={editingAttrs.name} />
      </label>
      <label class="se-field">
        <span>Podpowiedź dla LLM (hint)</span>
        <input bind:value={editingAttrs.hint} />
      </label>

      {#if editingAttrs.slotKind === "pick"}
        <div class="se-field">
          <span>Opcje</span>
          <div class="opts">
            {#each editingOptions as opt, i}
              <div class="opt-row">
                <input class="opt-code" bind:value={editingOptions[i].code} />
                <input class="opt-text" bind:value={editingOptions[i].text} placeholder="Etykieta" />
                <button type="button" class="opt-remove" onclick={() => removePickOption(i)}>×</button>
              </div>
            {/each}
            <button type="button" class="opt-add" onclick={addPickOption}>+ Dodaj opcję</button>
          </div>
        </div>
        <label class="se-check">
          <input type="checkbox" checked={editingAttrs.allowOther === "true"} onchange={(e) => (editingAttrs.allowOther = (e.currentTarget as HTMLInputElement).checked ? "true" : "false")} />
          Pozwól na opis własny (other)
        </label>
      {/if}

      {#if editingAttrs.slotKind === "list"}
        <label class="se-check">
          <input type="checkbox" checked={editingAttrs.numbered === "true"} onchange={(e) => (editingAttrs.numbered = (e.currentTarget as HTMLInputElement).checked ? "true" : "false")} />
          Numerowana
        </label>
      {/if}

      <div class="se-actions">
        <button type="button" class="btn btn-danger-ghost" onclick={deleteSlot}>Usuń</button>
        <span class="spacer"></span>
        <button type="button" class="btn btn-ghost" onclick={cancelSlotEdit}>Anuluj</button>
        <button type="button" class="btn btn-solid" onclick={commitSlotEdit}>Zapisz</button>
      </div>
    </div>
  {/if}
</section>

<style>
  .editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
    position: relative;
  }
  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  h2 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    flex: 1;
    text-align: center;
  }
  .arrow { font-size: 14px; }
  .form {
    flex: 1;
    overflow-y: auto;
    padding: 18px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .label { font-size: 12px; font-weight: 600; color: var(--text); }
  .hint { font-size: 11px; color: var(--text-muted); line-height: 1.4; margin-bottom: 4px; }
  input[type="text"], textarea {
    padding: 8px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 13px;
    color: var(--text);
    font-family: inherit;
  }
  input[type="text"]:focus, textarea:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .tiptap-wrap {
    position: relative;
  }
  .tiptap {
    min-height: 220px;
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 13px;
    line-height: 1.55;
  }
  .tiptap :global(.ProseMirror) {
    outline: none;
    min-height: 200px;
  }
  .tiptap :global(.ProseMirror h1) {
    font-size: 15px;
    font-weight: 700;
    margin: 12px 0 4px;
  }
  .tiptap :global(.ProseMirror h2) {
    font-size: 13px;
    font-weight: 600;
    margin: 10px 0 2px;
  }
  .tiptap :global(.slot-chip-edit) {
    display: inline-block;
    padding: 1px 6px;
    margin: 0 2px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-radius: 3px;
    color: var(--accent-text);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    user-select: none;
  }
  .slash-menu {
    position: absolute;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    padding: 4px;
    min-width: 220px;
    z-index: 20;
    display: flex;
    flex-direction: column;
  }
  .slash-item {
    padding: 6px 10px;
    background: transparent;
    border: none;
    text-align: left;
    font: inherit;
    color: inherit;
    border-radius: 4px;
    cursor: pointer;
  }
  .slash-item.active,
  .slash-item:hover {
    background: var(--bg-subtle);
  }
  .error {
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    color: var(--danger);
    font-size: 12px;
  }
  .spacer { flex: 1; }

  .slot-editor-backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--gray-12) 30%, transparent);
    z-index: 30;
  }
  .slot-editor {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 100%;
    max-width: 420px;
    max-height: 80vh;
    overflow-y: auto;
    z-index: 31;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-md);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .slot-editor h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    margin: 0;
  }
  .se-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
  }
  .se-field > span {
    font-weight: 600;
    color: var(--text);
  }
  .se-field input {
    padding: 6px 8px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 13px;
    font-family: inherit;
  }
  .se-field input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .se-check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .opts { display: flex; flex-direction: column; gap: 4px; }
  .opt-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .opt-code,
  .opt-text {
    padding: 4px 6px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    font: inherit;
  }
  .opt-code {
    width: 50px;
    text-align: center;
    font-weight: 600;
  }
  .opt-text {
    flex: 1;
  }
  .opt-code:focus,
  .opt-text:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .opt-remove {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    padding: 0 4px;
  }
  .opt-add {
    padding: 4px;
    background: transparent;
    border: 1px dashed var(--border);
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 12px;
    margin-top: 4px;
  }
  .se-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    border-top: 1px solid var(--border);
    padding-top: 10px;
  }
</style>
