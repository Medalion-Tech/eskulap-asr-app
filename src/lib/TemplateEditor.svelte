<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { templates } from "./stores";
  import type { Template } from "./stores";

  interface Props {
    template: Template | null;
    onclose: () => void;
  }

  let { template, onclose }: Props = $props();

  const isNew = $derived(template === null || template.id === "");

  let name = $state(template?.name ?? "");
  let description = $state(template?.description ?? "");
  let content = $state(template?.content ?? "");
  let exampleInput = $state(template?.example_input ?? "");
  let exampleOutput = $state(template?.example_output ?? "");
  let error = $state("");
  let saving = $state(false);

  async function save() {
    error = "";
    if (!name.trim()) {
      error = "Nazwa szablonu jest wymagana.";
      return;
    }
    if (!content.trim()) {
      error = "Treść szablonu jest wymagana.";
      return;
    }

    saving = true;
    try {
      const payload: Template = {
        id: template?.id ?? "",
        name: name.trim(),
        description: description.trim(),
        content: content.trim(),
        example_input: exampleInput.trim() ? exampleInput.trim() : null,
        example_output: exampleOutput.trim() ? exampleOutput.trim() : null,
        is_builtin: false,
        created_at: template?.created_at ?? "",
        updated_at: template?.updated_at ?? "",
      };

      if (isNew) {
        const created = await invoke<Template>("add_template", {
          template: payload,
        });
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
      error = e?.toString() ?? "Nie udało się zapisać szablonu";
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
    <h2>{isNew ? "Nowy szablon" : "Edycja szablonu"}</h2>
    <button class="btn btn-solid" onclick={save} disabled={saving}>
      {saving ? "Zapisywanie…" : "Zapisz"}
    </button>
  </header>

  <div class="form">
    <label class="field">
      <span class="label">Nazwa</span>
      <input
        type="text"
        bind:value={name}
        placeholder="np. Wizyta SOAP pediatryczna"
      />
    </label>

    <label class="field">
      <span class="label">Opis (opcjonalnie)</span>
      <input
        type="text"
        bind:value={description}
        placeholder="Krótki opis zastosowania"
      />
    </label>

    <label class="field">
      <span class="label">Treść szablonu</span>
      <span class="hint">
        Opisz co notatka ma zawierać i jaką ma mieć strukturę. Uniwersalne zasady
        (liczby&nbsp;→&nbsp;cyfry, kody ICD-10, łacińska terminologia, brak halucynacji)
        są już wbudowane — nie musisz ich powtarzać.
      </span>
      <textarea
        bind:value={content}
        rows="14"
        placeholder="Przekształć dyktowanie w notatkę z wizyty SOAP.&#10;&#10;Struktura wyjściowa:&#10;&#10;WYWIAD&#10;  - Dolegliwość główna&#10;  - Historia choroby aktualnej&#10;&#10;BADANIE PRZEDMIOTOWE&#10;  ..."
      ></textarea>
    </label>

    <label class="field">
      <span class="label">Przykład wejścia — dyktowanie (opcjonalnie)</span>
      <span class="hint">Poprawia jakość (few-shot). Wzorcowa surowa wypowiedź lekarza.</span>
      <textarea bind:value={exampleInput} rows="6" placeholder="Pacjent zgłasza się z..."></textarea>
    </label>

    <label class="field">
      <span class="label">Przykład wyjścia — gotowa notatka (opcjonalnie)</span>
      <span class="hint">Wzorcowa, sformatowana notatka dla powyższego dyktowania.</span>
      <textarea bind:value={exampleOutput} rows="6" placeholder="WYWIAD&#10;..."></textarea>
    </label>

    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>
</section>

<style>
  .editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
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

  .arrow {
    font-size: 14px;
  }

  .form {
    flex: 1;
    overflow-y: auto;
    padding: 18px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.005em;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
    margin-bottom: 2px;
  }

  input[type="text"],
  textarea {
    padding: 8px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 13px;
    color: var(--text);
    font-family: inherit;
    transition: border-color var(--duration-fast) var(--easing),
      box-shadow var(--duration-fast) var(--easing);
  }

  input[type="text"]:focus,
  textarea:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 15%, transparent);
  }

  textarea {
    resize: vertical;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    line-height: 1.5;
    min-height: 60px;
  }

  .error {
    padding: 8px 12px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    color: var(--danger);
    font-size: 12px;
  }
</style>
