<script lang="ts">
  import AppHeader from "./AppHeader.svelte";
  import RecordButton from "./RecordButton.svelte";
  import NotesList from "./NotesList.svelte";
  import { statusMessage } from "./stores";
</script>

<div class="main">
  <AppHeader />
  <RecordButton />
  <NotesList />
  {#if $statusMessage}
    <div class="toast">{$statusMessage}</div>
  {/if}
</div>

<style>
  .main {
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
    background: var(--bg);
  }

  .toast {
    position: absolute;
    bottom: 72px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    background: var(--gray-12);
    color: var(--gray-1);
    border-radius: 20px;
    font-size: 13px;
    font-weight: 500;
    box-shadow: var(--shadow-md);
    pointer-events: none;
    animation: toast-in 0.2s ease-out;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0);
    }
  }
</style>
