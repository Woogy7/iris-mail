<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sidebar from '$lib/components/sidebar/Sidebar.svelte';
  import MessageList from '$lib/components/message-list/MessageList.svelte';
  import ReadingPane from '$lib/components/reading-pane/ReadingPane.svelte';
  import { loadAccounts } from '$lib/stores/accounts.svelte';
  import { triggerFullSync, startPeriodicSync, stopPeriodicSync, selectAccount } from '$lib/stores/mail.svelte';
  import { getAccounts } from '$lib/stores/accounts.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';

  onMount(async () => {
    await loadAccounts();
    // Auto-select the first account so folders show immediately on launch.
    const accounts = getAccounts();
    if (accounts.length > 0) {
      await selectAccount(accounts[0].id);
    }
    // Sync all accounts on app launch.
    triggerFullSync();
    // Start periodic background sync (every 2 minutes).
    startPeriodicSync();
  });

  onDestroy(() => {
    stopPeriodicSync();
  });
</script>

<div class="flex flex-col h-screen overflow-hidden">
  <div class="grid grid-cols-[250px_350px_1fr] flex-1 min-h-0">
    <Sidebar />
    <MessageList />
    <ReadingPane />
  </div>
  <StatusBar />
</div>
