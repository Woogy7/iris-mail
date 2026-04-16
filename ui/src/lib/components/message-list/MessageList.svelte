<script lang="ts">
  import {
    getMessages, getSelectedFolderId, getSelectedMessageId,
    getIsLoadingMessages, selectMessage
  } from '$lib/stores/mail.svelte';
  import MessageRow from './MessageRow.svelte';

  let messages = $derived(getMessages());
  let selectedFolderId = $derived(getSelectedFolderId());
  let selectedMessageId = $derived(getSelectedMessageId());
  let isLoading = $derived(getIsLoadingMessages());
</script>

<section class="flex flex-col h-full min-h-0 border-l border-ctp-surface0 bg-ctp-base">
  {#if !selectedFolderId}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0 text-center">Select a folder</p>
    </div>
  {:else if isLoading}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0 text-center">Loading messages...</p>
    </div>
  {:else if messages.length === 0}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0 text-center">No messages in this folder</p>
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto">
      {#each messages as message (message.id)}
        <MessageRow
          {message}
          isSelected={message.id === selectedMessageId}
          onSelect={() => selectMessage(message.id)}
        />
      {/each}
    </div>
  {/if}
</section>
