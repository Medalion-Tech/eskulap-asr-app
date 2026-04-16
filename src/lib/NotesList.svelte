<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { notes, statusMessage } from "./stores";

  let compiled = $state("");
  let showCompiled = $state(false);

  const selectedCount = $derived($notes.filter((n) => n.selected).length);
  const allSelected = $derived(
    $notes.length > 0 && $notes.every((n) => n.selected)
  );

  function toggleSelect(id: string) {
    $notes = $notes.map((n) =>
      n.id === id ? { ...n, selected: !n.selected } : n
    );
  }

  function toggleSelectAll() {
    const target = !allSelected;
    $notes = $notes.map((n) => ({ ...n, selected: target }));
  }

  async function deleteNote(id: string, e: MouseEvent) {
    e.stopPropagation();
    await invoke("delete_note", { id });
    $notes = $notes.filter((n) => n.id !== id);
  }

  function getTitle(text: string): string {
    const firstLine = text.split("\n")[0].trim();
    return firstLine.length > 70 ? firstLine.slice(0, 70) + "…" : firstLine;
  }

  function getPreview(text: string): string {
    const lines = text.split("\n").slice(1);
    const rest = lines.join(" ").trim();
    if (rest) return rest;
    return text.length > 70 ? text.slice(70) : "";
  }

  function formatTime(ts: string): string {
    const [_, time] = ts.split(" ");
    return time ? time.slice(0, 5) : ts;
  }

  function compileNotes() {
    const selected = $notes.filter((n) => n.selected);
    if (selected.length === 0) return;
    compiled = selected.map((n) => n.text).join("\n\n");
    showCompiled = true;
  }

  async function copyCompiled() {
    await writeText(compiled);
    $statusMessage = "Skopiowano";
    setTimeout(() => ($statusMessage = ""), 1500);
  }

  async function copySelected() {
    const text = $notes
      .filter((n) => n.selected)
      .map((n) => n.text)
      .join("\n\n");
    if (!text) return;
    await writeText(text);
    $statusMessage = "Skopiowano";
    setTimeout(() => ($statusMessage = ""), 1500);
  }

  async function clearAll() {
    if (!confirm("Usunąć wszystkie notatki?")) return;
    await invoke("clear_notes");
    $notes = [];
    showCompiled = false;
    compiled = "";
  }
</script>

{#if showCompiled}
  <section class="compiled">
    <header class="compiled-header">
      <button class="btn btn-ghost" onclick={() => (showCompiled = false)}>
        <span class="arrow">←</span> Powrót
      </button>
      <button class="btn btn-solid" onclick={copyCompiled}>
        Kopiuj do schowka
      </button>
    </header>
    <div class="compiled-body">{compiled}</div>
  </section>
{:else}
  <section class="notes">
    <header class="notes-header">
      <h2>Notatki</h2>
      <div class="header-meta">
        {#if $notes.length > 0}
          <span class="count">{$notes.length}</span>
          <button class="btn btn-ghost btn-sm" onclick={toggleSelectAll}>
            {allSelected ? "Odznacz wszystkie" : "Zaznacz wszystkie"}
          </button>
        {/if}
      </div>
    </header>

    <div class="notes-scroll">
      {#if $notes.length === 0}
        <div class="empty">
          <div class="empty-icon">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
              <line x1="8" y1="13" x2="16" y2="13"/>
              <line x1="8" y1="17" x2="14" y2="17"/>
            </svg>
          </div>
          <div class="empty-text">Brak notatek</div>
          <div class="empty-hint">Nagraj pierwszą notatkę głosową</div>
        </div>
      {:else}
        <ul class="notes-list">
          {#each $notes as note (note.id)}
            <li class="note-item">
              <div
                class="note"
                class:selected={note.selected}
                role="button"
                tabindex="0"
                onclick={() => toggleSelect(note.id)}
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    toggleSelect(note.id);
                  }
                }}
              >
                <span class="note-check" class:checked={note.selected}>
                  {#if note.selected}
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                  {/if}
                </span>
                <div class="note-body">
                  <div class="note-row">
                    <span class="note-title">{getTitle(note.text)}</span>
                    <span class="note-time">{formatTime(note.timestamp)}</span>
                  </div>
                  {#if getPreview(note.text)}
                    <div class="note-preview">{getPreview(note.text)}</div>
                  {/if}
                </div>
                <button
                  class="note-delete"
                  onclick={(e) => deleteNote(note.id, e)}
                  aria-label="Usuń"
                  title="Usuń"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18"/>
                    <line x1="6" y1="6" x2="18" y2="18"/>
                  </svg>
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    {#if $notes.length > 0}
      <footer class="actions">
        <button
          class="btn btn-solid"
          onclick={compileNotes}
          disabled={selectedCount === 0}
        >
          Kompiluj{selectedCount > 0 ? ` · ${selectedCount}` : ""}
        </button>
        <button
          class="btn btn-outline"
          onclick={copySelected}
          disabled={selectedCount === 0}
        >
          Kopiuj
        </button>
        <div class="spacer"></div>
        <button class="btn btn-danger-ghost" onclick={clearAll}>
          Wyczyść
        </button>
      </footer>
    {/if}
  </section>
{/if}

<style>
  .notes,
  .compiled {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-subtle);
    border-top: 1px solid var(--border);
  }

  .notes-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px 10px;
    flex-shrink: 0;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.015em;
    color: var(--text);
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .count {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    background: var(--bg-active);
    padding: 2px 8px;
    border-radius: 10px;
    font-weight: 500;
  }

  .btn-sm {
    padding: 4px 8px;
    height: 24px;
    font-size: 12px;
  }

  .notes-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0 10px 8px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 56px 16px;
    color: var(--text-muted);
    gap: 6px;
  }

  .empty-icon {
    color: var(--border-strong);
    margin-bottom: 4px;
  }

  .empty-text {
    font-size: 14px;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .empty-hint {
    font-size: 13px;
    color: var(--text-muted);
  }

  .notes-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .note-item {
    list-style: none;
  }

  .note {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    text-align: left;
    transition: background var(--duration-fast) var(--easing);
    position: relative;
    cursor: pointer;
    outline: none;
  }

  .note:hover {
    background: var(--bg-hover);
  }

  .note.selected {
    background: var(--accent-soft-bg);
  }

  .note:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .note-check {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1.5px solid var(--border-strong);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 1px;
    color: transparent;
    transition: all var(--duration-fast) var(--easing);
  }

  .note-check.checked {
    background: var(--accent);
    border-color: var(--accent);
    color: #ffffff;
  }

  .note-body {
    flex: 1;
    min-width: 0;
  }

  .note-row {
    display: flex;
    align-items: baseline;
    gap: 12px;
    justify-content: space-between;
  }

  .note-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
    letter-spacing: -0.005em;
  }

  .note-time {
    font-size: 12px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .note-preview {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
  }

  .note-delete {
    position: absolute;
    top: 50%;
    right: 10px;
    transform: translateY(-50%);
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: var(--surface);
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--easing),
      color var(--duration-fast) var(--easing),
      background var(--duration-fast) var(--easing);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
  }

  .note:hover .note-delete {
    opacity: 1;
  }

  .note-delete:hover {
    background: var(--danger);
    color: #ffffff;
    border-color: var(--danger);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }

  .spacer {
    flex: 1;
  }

  /* Compiled view */
  .compiled-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }

  .arrow {
    font-size: 14px;
  }

  .compiled-body {
    flex: 1;
    padding: 24px;
    font-size: 14px;
    line-height: 1.65;
    white-space: pre-wrap;
    overflow-y: auto;
    color: var(--text);
    letter-spacing: -0.005em;
  }
</style>
