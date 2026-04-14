<script lang="ts">
  import { addM365Account, addImapAccount } from '$lib/api/accounts';
  import { loadAccounts } from '$lib/stores/accounts.svelte';

  let { isOpen, onClose }: {
    isOpen: boolean;
    onClose: () => void;
  } = $props();

  let step = $state<'choose' | 'm365' | 'imap'>('choose');
  let email = $state('');
  let displayName = $state('');
  let password = $state('');
  let isSubmitting = $state(false);
  let error = $state<string | null>(null);

  function reset() {
    step = 'choose';
    email = '';
    displayName = '';
    password = '';
    isSubmitting = false;
    error = null;
  }

  function handleClose() {
    reset();
    onClose();
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      handleClose();
    }
  }

  function handleBackdropKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      handleClose();
    }
  }

  async function handleM365Submit() {
    if (!email.trim() || !displayName.trim()) return;
    isSubmitting = true;
    error = null;
    try {
      await addM365Account(email.trim(), displayName.trim());
      await loadAccounts();
      handleClose();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleImapSubmit() {
    if (!email.trim() || !displayName.trim() || !password.trim()) return;
    isSubmitting = true;
    error = null;
    try {
      await addImapAccount({
        emailAddress: email.trim(),
        displayName: displayName.trim(),
        password: password.trim(),
      });
      await loadAccounts();
      handleClose();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      isSubmitting = false;
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center"
    onclick={handleBackdropClick}
    onkeydown={handleBackdropKeydown}
  >
    <div class="bg-ctp-base border border-ctp-surface0 rounded-lg p-6 w-96 shadow-xl">
      {#if step === 'choose'}
        <h2 class="text-lg font-semibold text-ctp-text mb-4">Add Account</h2>
        <div class="flex flex-col gap-3">
          <button
            class="bg-ctp-surface0 text-ctp-text px-4 py-3 rounded-md hover:bg-ctp-surface1 text-left transition-colors"
            onclick={() => { step = 'm365'; }}
          >
            <span class="font-medium">Microsoft 365</span>
            <span class="block text-xs text-ctp-subtext0 mt-0.5">Outlook, Hotmail, Live</span>
          </button>
          <button
            class="bg-ctp-surface0 text-ctp-text px-4 py-3 rounded-md hover:bg-ctp-surface1 text-left transition-colors"
            onclick={() => { step = 'imap'; }}
          >
            <span class="font-medium">Other (IMAP)</span>
            <span class="block text-xs text-ctp-subtext0 mt-0.5">Gmail, Yahoo, self-hosted</span>
          </button>
        </div>
        <button
          class="mt-4 text-sm text-ctp-subtext0 hover:text-ctp-text transition-colors"
          onclick={handleClose}
        >
          Cancel
        </button>

      {:else if step === 'm365'}
        <h2 class="text-lg font-semibold text-ctp-text mb-4">Microsoft 365</h2>
        <form class="flex flex-col gap-3" onsubmit={handleM365Submit}>
          <label class="flex flex-col gap-1">
            <span class="text-sm text-ctp-subtext1">Email address</span>
            <input
              type="email"
              bind:value={email}
              placeholder="you@outlook.com"
              required
              class="bg-ctp-surface0 border border-ctp-surface1 text-ctp-text px-3 py-2 rounded-md w-full
                     focus:outline-none focus:border-ctp-mauve placeholder:text-ctp-overlay0"
            />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-sm text-ctp-subtext1">Display name</span>
            <input
              type="text"
              bind:value={displayName}
              placeholder="Work"
              required
              class="bg-ctp-surface0 border border-ctp-surface1 text-ctp-text px-3 py-2 rounded-md w-full
                     focus:outline-none focus:border-ctp-mauve placeholder:text-ctp-overlay0"
            />
          </label>
          {#if error}
            <p class="text-ctp-red text-sm">{error}</p>
          {/if}
          <div class="flex gap-2 mt-2">
            <button
              type="button"
              class="bg-ctp-surface0 text-ctp-text px-4 py-2 rounded-md hover:bg-ctp-surface1 transition-colors"
              onclick={() => { step = 'choose'; error = null; }}
            >
              Back
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              class="bg-ctp-mauve text-ctp-base font-medium px-4 py-2 rounded-md hover:opacity-90
                     disabled:opacity-50 disabled:cursor-not-allowed flex-1 transition-opacity"
            >
              {isSubmitting ? 'Connecting...' : 'Sign in with Microsoft'}
            </button>
          </div>
        </form>

      {:else if step === 'imap'}
        <h2 class="text-lg font-semibold text-ctp-text mb-4">IMAP Account</h2>
        <form class="flex flex-col gap-3" onsubmit={handleImapSubmit}>
          <label class="flex flex-col gap-1">
            <span class="text-sm text-ctp-subtext1">Email address</span>
            <input
              type="email"
              bind:value={email}
              placeholder="you@example.com"
              required
              class="bg-ctp-surface0 border border-ctp-surface1 text-ctp-text px-3 py-2 rounded-md w-full
                     focus:outline-none focus:border-ctp-mauve placeholder:text-ctp-overlay0"
            />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-sm text-ctp-subtext1">Display name</span>
            <input
              type="text"
              bind:value={displayName}
              placeholder="Personal"
              required
              class="bg-ctp-surface0 border border-ctp-surface1 text-ctp-text px-3 py-2 rounded-md w-full
                     focus:outline-none focus:border-ctp-mauve placeholder:text-ctp-overlay0"
            />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-sm text-ctp-subtext1">Password</span>
            <input
              type="password"
              bind:value={password}
              required
              class="bg-ctp-surface0 border border-ctp-surface1 text-ctp-text px-3 py-2 rounded-md w-full
                     focus:outline-none focus:border-ctp-mauve placeholder:text-ctp-overlay0"
            />
          </label>
          {#if error}
            <p class="text-ctp-red text-sm">{error}</p>
          {/if}
          <div class="flex gap-2 mt-2">
            <button
              type="button"
              class="bg-ctp-surface0 text-ctp-text px-4 py-2 rounded-md hover:bg-ctp-surface1 transition-colors"
              onclick={() => { step = 'choose'; error = null; }}
            >
              Back
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              class="bg-ctp-mauve text-ctp-base font-medium px-4 py-2 rounded-md hover:opacity-90
                     disabled:opacity-50 disabled:cursor-not-allowed flex-1 transition-opacity"
            >
              {isSubmitting ? 'Connecting...' : 'Connect'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}
