<script lang="ts">
  import { tick } from 'svelte';
  import {
    getMessages, getSelectedFolderId, getSelectedMessageId,
    getIsLoadingMessages, selectMessage
  } from '$lib/stores/mail.svelte';
  import MessageRow from './MessageRow.svelte';

  let messages = $derived(getMessages());
  let selectedFolderId = $derived(getSelectedFolderId());
  let selectedMessageId = $derived(getSelectedMessageId());
  let isLoading = $derived(getIsLoadingMessages());

  let listEl = $state<HTMLDivElement | null>(null);

  /// Select a message and scroll its row into view if needed.
  async function selectAndReveal(messageId: string) {
    selectMessage(messageId);
    await tick();
    const row = listEl?.querySelector<HTMLElement>(
      `[data-message-id="${CSS.escape(messageId)}"]`
    );
    row?.scrollIntoView({ block: 'nearest' });
  }

  /// Arrow-key navigation. Wrap-around is intentionally NOT supported:
  /// at the ends, the key is a no-op (standard mail-client behaviour).
  /// With no current selection, ArrowDown picks the first message and
  /// ArrowUp picks the last — this lets the user "enter" the list from
  /// either end depending on intent.
  function handleKeydown(event: KeyboardEvent) {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    if (messages.length === 0) return;

    const currentIndex = selectedMessageId
      ? messages.findIndex(m => m.id === selectedMessageId)
      : -1;

    let nextIndex: number;
    if (event.key === 'ArrowDown') {
      if (currentIndex === -1) {
        nextIndex = 0;
      } else if (currentIndex >= messages.length - 1) {
        event.preventDefault();
        return;
      } else {
        nextIndex = currentIndex + 1;
      }
    } else {
      if (currentIndex === -1) {
        nextIndex = messages.length - 1;
      } else if (currentIndex <= 0) {
        event.preventDefault();
        return;
      } else {
        nextIndex = currentIndex - 1;
      }
    }

    event.preventDefault();
    selectAndReveal(messages[nextIndex].id);
  }

  /// Pull focus to the list pane when a row is clicked, so subsequent
  /// arrow keys are routed here instead of being swallowed by the body.
  function handleRowSelect(messageId: string) {
    selectMessage(messageId);
    listEl?.focus({ preventScroll: true });
  }
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
    <div
      bind:this={listEl}
      class="flex-1 overflow-y-auto focus:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-ctp-overlay0"
      tabindex="0"
      role="listbox"
      aria-label="Messages"
      onkeydown={handleKeydown}
    >
      {#each messages as message (message.id)}
        <MessageRow
          {message}
          isSelected={message.id === selectedMessageId}
          onSelect={() => handleRowSelect(message.id)}
        />
      {/each}
    </div>
  {/if}
</section>
