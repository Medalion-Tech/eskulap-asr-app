<script lang="ts">
  import { screen } from "./stores";
  import TemplateList from "./TemplateList.svelte";

  function back() {
    $screen = "main";
  }
</script>

<div class="templates-screen">
  <header class="header">
    <button class="back-btn" onclick={back} aria-label="Wróć">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>
    <h1>Szablony</h1>
    <div class="spacer"></div>
  </header>

  <div class="body">
    <TemplateList />
  </div>
</div>

<style>
  .templates-screen {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 16px 6px 84px;
    height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    -webkit-app-region: drag;
    app-region: drag;
    flex-shrink: 0;
  }

  .back-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    -webkit-app-region: no-drag;
    app-region: no-drag;
    transition: background var(--duration-fast) var(--easing);
  }

  .back-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  h1 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .spacer {
    flex: 1;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
