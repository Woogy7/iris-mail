import { listFolders, syncFolders, type Folder } from '$lib/api/folders';
import {
  listMessages,
  getMessageBody,
  syncAccount,
  syncAllAccounts,
  type Message,
  type MessageBody,
} from '$lib/api/messages';

let selectedAccountId = $state<string | null>(null);
let folders = $state<Folder[]>([]);
let selectedFolderId = $state<string | null>(null);
let messages = $state<Message[]>([]);
let selectedMessageId = $state<string | null>(null);
let messageBody = $state<MessageBody | null>(null);
let isLoadingFolders = $state(false);
let isLoadingMessages = $state(false);
let isLoadingBody = $state(false);
let isSyncing = $state(false);

let syncIntervalId: ReturnType<typeof setInterval> | null = null;

// --- Getters ---

export function getSelectedAccountId(): string | null {
  return selectedAccountId;
}

export function getFolders(): Folder[] {
  return folders;
}

export function getSelectedFolderId(): string | null {
  return selectedFolderId;
}

export function getMessages(): Message[] {
  return messages;
}

export function getSelectedMessageId(): string | null {
  return selectedMessageId;
}

export function getMessageBodyData(): MessageBody | null {
  return messageBody;
}

export function getIsLoadingFolders(): boolean {
  return isLoadingFolders;
}

export function getIsLoadingMessages(): boolean {
  return isLoadingMessages;
}

export function getIsLoadingBody(): boolean {
  return isLoadingBody;
}

export function getIsSyncing(): boolean {
  return isSyncing;
}

// --- Actions ---

export async function selectAccount(accountId: string) {
  selectedAccountId = accountId;
  selectedFolderId = null;
  selectedMessageId = null;
  messageBody = null;
  messages = [];
  isLoadingFolders = true;
  try {
    folders = await listFolders(accountId);
  } catch (e) {
    console.error('Failed to load folders:', e);
  } finally {
    isLoadingFolders = false;
  }
}

export async function selectFolder(folderId: string) {
  selectedFolderId = folderId;
  selectedMessageId = null;
  messageBody = null;
  isLoadingMessages = true;
  try {
    messages = await listMessages(folderId);
  } catch (e) {
    console.error('Failed to load messages:', e);
  } finally {
    isLoadingMessages = false;
  }
}

export async function selectMessage(messageId: string) {
  selectedMessageId = messageId;
  isLoadingBody = true;
  try {
    messageBody = await getMessageBody(messageId);
  } catch (e) {
    console.error('Failed to load message body:', e);
  } finally {
    isLoadingBody = false;
  }
}

/// Full sync for a single account: folders + messages for all folders.
export async function triggerAccountSync(accountId: string) {
  isSyncing = true;
  try {
    await syncAccount(accountId);
    // Refresh local state after sync.
    if (selectedAccountId === accountId) {
      folders = await listFolders(accountId);
      if (selectedFolderId) {
        messages = await listMessages(selectedFolderId);
      }
    }
  } catch (e) {
    console.error('Account sync failed:', e);
  } finally {
    isSyncing = false;
  }
}

/// Sync all accounts (used on app launch and periodic sync).
export async function triggerFullSync() {
  isSyncing = true;
  try {
    await syncAllAccounts();
    // Refresh local state after sync.
    if (selectedAccountId) {
      folders = await listFolders(selectedAccountId);
      if (selectedFolderId) {
        messages = await listMessages(selectedFolderId);
      }
    }
  } catch (e) {
    console.error('Full sync failed:', e);
  } finally {
    isSyncing = false;
  }
}

/// Legacy alias kept for sidebar sync button.
export async function triggerFolderSync(accountId: string) {
  return triggerAccountSync(accountId);
}

/// Start periodic background sync (every 2 minutes).
export function startPeriodicSync() {
  if (syncIntervalId) return; // Already running.
  syncIntervalId = setInterval(() => {
    triggerFullSync();
  }, 2 * 60 * 1000);
}

/// Stop the periodic background sync.
export function stopPeriodicSync() {
  if (syncIntervalId) {
    clearInterval(syncIntervalId);
    syncIntervalId = null;
  }
}
