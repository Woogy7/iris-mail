<script lang="ts">
  import {
    getSelectedMessageId, getMessageBodyData, getMessages, getIsLoadingBody
  } from '$lib/stores/mail.svelte';

  let selectedMessageId = $derived(getSelectedMessageId());
  let body = $derived(getMessageBodyData());
  let messages = $derived(getMessages());
  let isLoading = $derived(getIsLoadingBody());

  let selectedMessage = $derived(
    messages.find(m => m.id === selectedMessageId) ?? null
  );

  let showRemoteImages = $state(false);

  // Reset show-images when switching messages.
  $effect(() => {
    if (selectedMessageId) {
      showRemoteImages = false;
    }
  });

  // The body HTML with or without remote images.
  let displayHtml = $derived(() => {
    if (!body?.sanitised_html) return null;
    if (showRemoteImages) return body.html ?? body.sanitised_html;
    return body.sanitised_html;
  });

  function formatFullDate(dateStr: string | null): string {
    if (!dateStr) return '';
    const date = new Date(dateStr);
    return date.toLocaleDateString(undefined, {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }
</script>

<section class="flex flex-col h-full min-h-0 border-l border-ctp-surface0 bg-ctp-base">
  {#if !selectedMessageId}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0 text-center">Select a message to read</p>
    </div>
  {:else if isLoading}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0 text-center">Loading...</p>
    </div>
  {:else if selectedMessage}
    <div class="bg-ctp-mantle px-4 py-3">
      <h2 class="text-lg font-semibold text-ctp-text">
        {selectedMessage.subject || '(no subject)'}
      </h2>
      <p class="text-sm text-ctp-subtext1 mt-1">
        From: {selectedMessage.from_name ? `${selectedMessage.from_name} <${selectedMessage.from_address}>` : selectedMessage.from_address || 'Unknown'}
      </p>
      {#if selectedMessage.to_addresses}
        <p class="text-sm text-ctp-subtext0 mt-0.5">
          To: {selectedMessage.to_addresses}
        </p>
      {/if}
      <p class="text-sm text-ctp-subtext0 mt-0.5">
        {formatFullDate(selectedMessage.date)}
      </p>
    </div>

    {#if body?.sanitised_html && !showRemoteImages}
      <div class="bg-ctp-surface0 text-ctp-subtext0 text-xs px-4 py-2 flex items-center justify-between shrink-0">
        <span>Remote images are blocked</span>
        <button
          class="text-ctp-mauve hover:text-ctp-text transition-colors"
          onclick={() => { showRemoteImages = true; }}
        >
          Show images
        </button>
      </div>
    {/if}

    <div class="h-px bg-ctp-surface0 shrink-0"></div>

    <div class="flex-1 overflow-y-auto min-h-0">
      {#if body?.sanitised_html || body?.html}
        <div class="p-4 prose-email">
          {@html displayHtml()}
        </div>
      {:else if body?.plain_text}
        <pre class="whitespace-pre-wrap font-mono text-sm text-ctp-text p-4">{body.plain_text}</pre>
      {:else}
        <div class="flex flex-1 items-center justify-center px-4 py-8">
          <p class="text-sm text-ctp-overlay0 text-center">Message body not available</p>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .prose-email {
    color: var(--color-ctp-text);
    font-size: 0.875rem;
    line-height: 1.625;
    max-width: 72ch;
  }

  .prose-email :global(a) {
    color: var(--color-ctp-sapphire);
    text-decoration: underline;
  }

  .prose-email :global(a:hover) {
    color: var(--color-ctp-blue);
  }

  .prose-email :global(p) {
    margin-top: 0.5em;
    margin-bottom: 0.5em;
  }

  .prose-email :global(h1),
  .prose-email :global(h2),
  .prose-email :global(h3) {
    color: var(--color-ctp-text);
    margin-top: 1em;
    margin-bottom: 0.5em;
    font-weight: 600;
  }

  .prose-email :global(blockquote) {
    border-left: 3px solid var(--color-ctp-surface1);
    padding-left: 1em;
    color: var(--color-ctp-subtext0);
    margin: 0.5em 0;
  }

  .prose-email :global(img) {
    max-width: 100%;
    height: auto;
  }

  .prose-email :global(pre) {
    background-color: var(--color-ctp-mantle);
    padding: 0.75em;
    border-radius: 0.375rem;
    overflow-x: auto;
    font-size: 0.8125rem;
  }

  .prose-email :global(code) {
    font-family: var(--font-mono);
  }

  .prose-email :global(ul),
  .prose-email :global(ol) {
    padding-left: 1.5em;
    margin: 0.5em 0;
  }

  .prose-email :global(table) {
    border-collapse: collapse;
    width: 100%;
  }

  .prose-email :global(td),
  .prose-email :global(th) {
    padding: 0.25em 0.5em;
    border: 1px solid var(--color-ctp-surface1);
  }
</style>
