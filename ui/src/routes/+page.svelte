<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/sidebar/Sidebar.svelte';
  import MessageList from '$lib/components/message-list/MessageList.svelte';
  import ReadingPane from '$lib/components/reading-pane/ReadingPane.svelte';
  import { loadAccounts } from '$lib/stores/accounts.svelte';

  onMount(() => {
    loadAccounts();
  });
</script>

<div class="grid grid-cols-[250px_350px_1fr] h-screen">
  <Sidebar />
  <MessageList />
  <ReadingPane />
</div>
