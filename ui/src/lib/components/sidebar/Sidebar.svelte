<script lang="ts">
  import type { AccentColour } from '$lib/api/accounts';
  import { getAccounts, getIsLoading } from '$lib/stores/accounts.svelte';
  import {
    getFolders,
    getSelectedAccountId,
    getSelectedFolderId,
    getIsLoadingFolders,
    selectAccount,
    selectFolder,
    triggerFolderSync,
  } from '$lib/stores/mail.svelte';
  import FolderTree from './FolderTree.svelte';
  import AddAccountDialog from './AddAccountDialog.svelte';

  let showAddAccount = $state(false);

  let accounts = $derived(getAccounts());
  let folders = $derived(getFolders());
  let selectedAccountId = $derived(getSelectedAccountId());
  let selectedFolderId = $derived(getSelectedFolderId());
  let isLoading = $derived(getIsLoading());
  let isLoadingFolders = $derived(getIsLoadingFolders());

  const accentColors: Record<AccentColour, string> = {
    Red: 'bg-ctp-red',
    Peach: 'bg-ctp-peach',
    Yellow: 'bg-ctp-yellow',
    Green: 'bg-ctp-green',
    Sapphire: 'bg-ctp-sapphire',
    Mauve: 'bg-ctp-mauve',
    Lavender: 'bg-ctp-lavender',
  };

  function handleAccountClick(accountId: string) {
    selectAccount(accountId);
  }

  function handleFolderSelected(folderId: string) {
    selectFolder(folderId);
  }

  function handleSync(event: MouseEvent, accountId: string) {
    event.stopPropagation();
    triggerFolderSync(accountId);
  }
</script>

<aside class="flex flex-col h-full bg-ctp-mantle">
  <div class="px-4 py-3">
    <h1 class="text-lg font-semibold text-ctp-mauve tracking-tight">
      Iris Mail
    </h1>
  </div>

  <div class="h-px bg-ctp-surface0"></div>

  {#if isLoading}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0">Loading accounts...</p>
    </div>
  {:else if accounts.length === 0}
    <div class="flex flex-1 items-center justify-center px-4">
      <p class="text-sm text-ctp-overlay0 text-center">
        No accounts configured
      </p>
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto">
      {#each accounts as account (account.id)}
        <div class="mt-1">
          <div
            class="flex items-center gap-2 px-3 py-2 transition-colors
                   {account.id === selectedAccountId ? 'bg-ctp-surface0/50' : 'hover:bg-ctp-surface0/30'}"
          >
            <button
              class="flex items-center gap-2 flex-1 min-w-0 text-left"
              onclick={() => handleAccountClick(account.id)}
            >
              <span class="w-2.5 h-2.5 rounded-full shrink-0 {accentColors[account.accent_colour] ?? 'bg-ctp-mauve'}"></span>
              <span class="text-sm font-medium text-ctp-text truncate">{account.display_name}</span>
            </button>
            {#if account.id === selectedAccountId}
              <button
                class="text-ctp-subtext0 hover:text-ctp-text transition-colors p-0.5 rounded shrink-0"
                title="Sync folders"
                onclick={(e) => handleSync(e, account.id)}
              >
                <svg class="w-3.5 h-3.5 {isLoadingFolders ? 'animate-spin' : ''}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M14 8A6 6 0 1 1 8 2" stroke-linecap="round" />
                  <path d="M8 0l2.5 2L8 4" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              </button>
            {/if}
          </div>

          {#if account.id === selectedAccountId}
            {#if isLoadingFolders}
              <div class="px-4 py-2">
                <p class="text-xs text-ctp-overlay0">Loading folders...</p>
              </div>
            {:else if folders.length > 0}
              <FolderTree
                {folders}
                {selectedFolderId}
                onFolderSelected={handleFolderSelected}
              />
            {:else}
              <div class="px-4 py-2">
                <p class="text-xs text-ctp-overlay0">No folders found</p>
              </div>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <div class="h-px bg-ctp-surface0"></div>

  <div class="px-3 py-3">
    <button
      class="w-full bg-ctp-surface0 text-ctp-text text-sm px-4 py-2 rounded-md hover:bg-ctp-surface1 transition-colors"
      onclick={() => { showAddAccount = true; }}
    >
      Add Account
    </button>
  </div>

  <AddAccountDialog
    isOpen={showAddAccount}
    onClose={() => { showAddAccount = false; }}
  />
</aside>
