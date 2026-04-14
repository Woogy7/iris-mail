<script lang="ts">
  import type { Message } from '$lib/api/messages';

  let { message, isSelected, onSelect }: {
    message: Message;
    isSelected: boolean;
    onSelect: () => void;
  } = $props();

  let isUnread = $derived(!message.flags.is_read);
  let senderDisplay = $derived(message.from_name || message.from_address || 'Unknown');
  let formattedDate = $derived(formatDate(message.date));

  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '';
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMinutes = Math.floor(diffMs / 60000);
    const diffHours = diffMs / (1000 * 60 * 60);

    if (diffMinutes < 1) return 'now';
    if (diffHours < 1) return `${diffMinutes}m`;
    if (diffHours < 24) return `${Math.floor(diffHours)}h`;
    if (diffHours < 24 * 7) {
      return date.toLocaleDateString(undefined, { weekday: 'short' });
    }
    if (date.getFullYear() === now.getFullYear()) {
      return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
    }
    return date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  }
</script>

<button
  class="w-full text-left px-3 py-2 flex items-start gap-2 border-b border-ctp-surface0/50 transition-colors
         {isSelected ? 'bg-ctp-surface0' : 'hover:bg-ctp-surface0/50'}"
  onclick={onSelect}
>
  <div class="mt-2 shrink-0 w-2 flex items-center justify-center">
    {#if isUnread}
      <span class="block w-1.5 h-1.5 rounded-full bg-ctp-mauve"></span>
    {/if}
  </div>

  <div class="flex-1 min-w-0">
    <div class="flex items-center justify-between gap-2">
      <span class="truncate text-sm {isUnread ? 'font-medium text-ctp-text' : 'font-normal text-ctp-subtext1'}">
        {senderDisplay}
      </span>
      <div class="flex items-center gap-1 shrink-0">
        {#if message.flags.is_flagged}
          <span class="text-xs text-ctp-yellow" title="Flagged">*</span>
        {/if}
        <span class="text-xs text-ctp-overlay0">{formattedDate}</span>
      </div>
    </div>
    <div class="truncate text-sm {isUnread ? 'text-ctp-text' : 'text-ctp-subtext1'}">
      {message.subject || '(no subject)'}
    </div>
  </div>
</button>
