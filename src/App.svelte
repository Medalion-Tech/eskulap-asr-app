<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { screen, notes, templates, statusMessage } from "./lib/stores";
  import SetupScreen from "./lib/SetupScreen.svelte";
  import MainScreen from "./lib/MainScreen.svelte";
  import SettingsScreen from "./lib/SettingsScreen.svelte";

  onMount(async () => {
    try {
      const whisperExists: boolean = await invoke("check_model_exists");
      const llmExists: boolean = await invoke("check_llm_model_exists");

      if (!whisperExists || !llmExists) {
        $screen = "setup";
        return;
      }

      $screen = "loading";
      $statusMessage = "Ładowanie modelu ASR…";
      await invoke("load_model");
      $statusMessage = "Ładowanie modelu LLM…";
      await invoke("load_llm_model");

      const savedNotes = await invoke<
        Array<{
          id: string;
          timestamp: string;
          text: string;
          raw_transcription?: string | null;
          template_id?: string | null;
          template_name?: string | null;
        }>
      >("get_notes");
      $notes = savedNotes.map((n) => ({ ...n, selected: false }));
      $templates = await invoke("get_templates");

      $statusMessage = "";
      $screen = "main";
    } catch (e) {
      console.error("Init error:", e);
      $statusMessage = "";
      $screen = "setup";
    }
  });
</script>

{#if $screen === "setup"}
  <SetupScreen />
{:else if $screen === "loading"}
  <div class="loading">
    <div class="titlebar-spacer"></div>
    <div class="loading-content">
      <div class="spinner"></div>
      <p>{$statusMessage || "Ładowanie modelu…"}</p>
    </div>
  </div>
{:else if $screen === "settings"}
  <SettingsScreen />
{:else}
  <MainScreen />
{/if}

<style>
  .loading {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .titlebar-spacer {
    height: 28px;
    -webkit-app-region: drag;
    app-region: drag;
  }

  .loading-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
