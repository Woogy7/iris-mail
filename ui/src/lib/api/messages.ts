import { invoke } from '@tauri-apps/api/core';

export interface MessageFlags {
  is_read: boolean;
  is_flagged: boolean;
  is_answered: boolean;
}

export interface Message {
  id: string;
  account_id: string;
  folder_id: string;
  uid: number | null;
  message_id_header: string | null;
  thread_id: string | null;
  subject: string | null;
  from_name: string | null;
  from_address: string | null;
  to_addresses: string | null;
  cc_addresses: string | null;
  bcc_addresses: string | null;
  date: string | null;
  size_bytes: number | null;
  flags: MessageFlags;
  is_stored_local: boolean;
  is_stored_remote: boolean;
  created_at: string;
  updated_at: string;
}

export interface MessageBody {
  message_id: string;
  html: string | null;
  sanitised_html: string | null;
  plain_text: string | null;
}

export async function listMessages(
  folderId: string,
  limit?: number,
  offset?: number,
): Promise<Message[]> {
  return invoke<Message[]>('list_messages', { folderId, limit, offset });
}

export async function fetchFolderMessages(
  accountId: string,
  folderId: string,
): Promise<Message[]> {
  return invoke<Message[]>('fetch_folder_messages', { accountId, folderId });
}

export async function getMessageBody(messageId: string): Promise<MessageBody> {
  return invoke<MessageBody>('get_message_body', { messageId });
}

export async function markMessageRead(messageId: string): Promise<void> {
  return invoke<void>('mark_message_read', { messageId });
}

export async function syncAccount(accountId: string): Promise<void> {
  return invoke<void>('sync_account', { accountId });
}

export async function syncAllAccounts(): Promise<void> {
  return invoke<void>('sync_all_accounts');
}
