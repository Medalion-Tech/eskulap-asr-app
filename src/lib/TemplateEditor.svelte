<script lang="ts">
  import type { Template } from "./stores";
  import TiptapEditor from "./editor/TiptapEditor.svelte";

  interface Props {
    template: Template | null;
    onclose: () => void;
  }

  let { template, onclose }: Props = $props();
</script>

<TiptapEditor {template} {onclose} />
