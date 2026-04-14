<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sidebar from '$lib/components/sidebar/Sidebar.svelte';
  import MessageList from '$lib/components/message-list/MessageList.svelte';
  import ReadingPane from '$lib/components/reading-pane/ReadingPane.svelte';
  import { loadAccounts } from '$lib/stores/accounts.svelte';
  import { triggerFullSync, startPeriodicSync, stopPeriodicSync } from '$lib/stores/mail.svelte';

  onMount(async () => {
    await loadAccounts();
    // Sync all accounts on app launch.
    triggerFullSync();
    // Start periodic background sync (every 2 minutes).
    startPeriodicSync();
  });

  onDestroy(() => {
    stopPeriodicSync();
  });
</script>

<div class="grid grid-cols-[250px_350px_1fr] h-screen">
  <Sidebar />
  <MessageList />
  <ReadingPane />
</div>
