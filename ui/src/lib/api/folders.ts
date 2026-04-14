import { invoke } from '@tauri-apps/api/core';

export type SpecialFolder = 'Inbox' | 'Sent' | 'Drafts' | 'Trash' | 'Archive' | 'Other';

export interface Folder {
  id: string;
  account_id: string;
  parent_id: string | null;
  name: string;
  full_path: string;
  special: SpecialFolder;
  uid_validity: number | null;
  last_seen_uid: number | null;
  message_count: number;
  unread_count: number;
  last_synced_at: string | null;
  created_at: string;
  updated_at: string;
}

export async function listFolders(accountId: string): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders', { accountId });
}

export async function syncFolders(accountId: string): Promise<Folder[]> {
  return invoke<Folder[]>('sync_folders', { accountId });
}
