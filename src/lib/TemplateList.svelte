<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { templates, selectedTemplateId, statusMessage } from "./stores";
  import type { Template } from "./stores";
  import TemplateEditor from "./TemplateEditor.svelte";

  let editorOpen = $state(false);
  let editing: Template | null = $state(null);

  function openNew() {
    editing = null;
    editorOpen = true;
  }

  function openEdit(t: Template) {
    editing = t;
    editorOpen = true;
  }

  async function duplicate(t: Template) {
    const copy: Template = {
      ...t,
      id: "",
      name: `${t.name} (kopia)`,
      is_builtin: false,
      created_at: "",
      updated_at: "",
      ast_version: 1,
    };
    const created = await invoke<Template>("add_template", { template: copy });
    $templates = [...$templates, created];
    editing = created;
    editorOpen = true;
  }

  function slotCount(t: Template): number {
    return Object.keys(t.ast?.slots ?? {}).length;
  }

  async function remove(t: Template) {
    if (t.is_builtin) return;
    if (!confirm(`Usunąć szablon „${t.name}"?`)) return;
    try {
      await invoke("delete_template", { id: t.id });
      $templates = $templates.filter((x) => x.id !== t.id);
      if ($selectedTemplateId === t.id) {
        $selectedTemplateId = null;
      }
      $statusMessage = "Szablon usunięty";
      setTimeout(() => ($statusMessage = ""), 1500);
    } catch (e: any) {
      alert(e?.toString() ?? "Nie udało się usunąć szablonu");
    }
  }

  function closeEditor() {
    editorOpen = false;
    editing = null;
  }
</script>

{#if editorOpen}
  <TemplateEditor template={editing} onclose={closeEditor} />
{:else}
  <section class="list">
    <header class="list-header">
      <h2>Szablony</h2>
      <button class="btn btn-solid btn-sm" onclick={openNew}>
        + Nowy szablon
      </button>
    </header>

    <div class="list-scroll">
      <ul>
        {#each $templates as t (t.id)}
          <li class="card">
            <div class="card-head">
              <div class="card-title">
                <span class="name">{t.name}</span>
                {#if t.is_builtin}
                  <span class="badge">wbudowany</span>
                {/if}
              </div>
              <div class="card-actions">
                {#if t.is_builtin}
                  <button class="btn btn-ghost btn-sm" onclick={() => duplicate(t)}>
                    Duplikuj
                  </button>
                {:else}
                  <button class="btn btn-ghost btn-sm" onclick={() => duplicate(t)}>
                    Duplikuj
                  </button>
                  <button class="btn btn-ghost btn-sm" onclick={() => openEdit(t)}>
                    Edytuj
                  </button>
                  <button class="btn btn-danger-ghost btn-sm" onclick={() => remove(t)}>
                    Usuń
                  </button>
                {/if}
              </div>
            </div>
            {#if t.description}
              <div class="desc">{t.description}</div>
            {/if}
            <div class="slot-count">{slotCount(t)} slotów</div>
          </li>
        {/each}
      </ul>
    </div>
  </section>
{/if}

<style>
  .list {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px 10px;
    flex-shrink: 0;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.015em;
  }

  .btn-sm {
    padding: 4px 10px;
    height: 26px;
    font-size: 12px;
  }

  .list-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0 16px 16px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .card {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .card-title {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.005em;
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

  .card-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .desc {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
    line-height: 1.5;
  }

  .slot-count {
    margin-top: 8px;
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
