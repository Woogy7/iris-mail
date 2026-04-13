<script lang="ts">
  import '../app.css';
  import { initTheme } from '$lib/stores/theme.svelte';

  initTheme();

  let { children } = $props();
</script>

{@render children()}
