import { listFolders, syncFolders, type Folder } from '$lib/api/folders';
import {
  listMessages,
  fetchFolderMessages,
  getMessageBody,
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

export async function triggerFolderSync(accountId: string) {
  isLoadingFolders = true;
  try {
    folders = await syncFolders(accountId);
  } catch (e) {
    console.error('Failed to sync folders:', e);
  } finally {
    isLoadingFolders = false;
  }
}
