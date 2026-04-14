<script lang="ts">
  import type { Folder, SpecialFolder } from '$lib/api/folders';

  let { folders, selectedFolderId, onFolderSelected }: {
    folders: Folder[];
    selectedFolderId: string | null;
    onFolderSelected: (folderId: string) => void;
  } = $props();

  const specialOrder: SpecialFolder[] = ['Inbox', 'Sent', 'Drafts', 'Trash', 'Archive'];

  let sortedFolders = $derived(
    [...folders].sort((a, b) => {
      const aIndex = specialOrder.indexOf(a.special);
      const bIndex = specialOrder.indexOf(b.special);
      const aIsSpecial = aIndex !== -1;
      const bIsSpecial = bIndex !== -1;

      if (aIsSpecial && bIsSpecial) return aIndex - bIndex;
      if (aIsSpecial) return -1;
      if (bIsSpecial) return 1;
      return a.name.localeCompare(b.name);
    })
  );
</script>

<div class="flex flex-col gap-0.5 px-2 py-1">
  {#each sortedFolders as folder (folder.id)}
    <button
      class="w-full text-left px-3 py-1.5 flex items-center justify-between rounded-md text-sm transition-colors
             {folder.id === selectedFolderId ? 'bg-ctp-surface0 text-ctp-text' : 'text-ctp-subtext1 hover:bg-ctp-surface0/50'}"
      onclick={() => onFolderSelected(folder.id)}
    >
      <span class="truncate">{folder.name}</span>
      {#if folder.unread_count > 0}
        <span class="text-xs text-ctp-mauve font-medium ml-2 shrink-0">{folder.unread_count}</span>
      {/if}
    </button>
  {/each}
</div>
