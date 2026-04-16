<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { screen, notes } from "./lib/stores";
  import SetupScreen from "./lib/SetupScreen.svelte";
  import MainScreen from "./lib/MainScreen.svelte";

  onMount(async () => {
    try {
      const exists: boolean = await invoke("check_model_exists");
      if (exists) {
        $screen = "loading";
        await invoke("load_model");
        const savedNotes = await invoke<
          Array<{ id: string; timestamp: string; text: string }>
        >("get_notes");
        $notes = savedNotes.map((n) => ({ ...n, selected: false }));
        $screen = "main";
      } else {
        $screen = "setup";
      }
    } catch (e) {
      console.error("Init error:", e);
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
      <p>Ładowanie modelu…</p>
    </div>
  </div>
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
