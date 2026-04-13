import { invoke } from '@tauri-apps/api/core';

export interface Folder {
  id: string;
  account_id: string;
  name: string;
  parent_id: string | null;
  special: string;
  message_count: number;
  unread_count: number;
}

export async function listFolders(accountId: string): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders', { accountId });
}
