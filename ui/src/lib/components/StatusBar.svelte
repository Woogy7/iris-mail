<script lang="ts">
  import { getIsSyncing } from '$lib/stores/mail.svelte';

  let isSyncing = $derived(getIsSyncing());
</script>

<footer class="flex items-center h-6 px-3 bg-ctp-mantle border-t border-ctp-surface0 text-xs text-ctp-subtext0 shrink-0">
  {#if isSyncing}
    <div class="flex items-center gap-1.5">
      <svg class="w-3 h-3 animate-spin text-ctp-mauve" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M14 8A6 6 0 1 1 8 2" stroke-linecap="round" />
      </svg>
      <span>Syncing...</span>
    </div>
  {:else}
    <span>Ready</span>
  {/if}
</footer>
