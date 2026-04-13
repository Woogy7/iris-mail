import { invoke } from '@tauri-apps/api/core';

export interface Message {
  id: string;
  folder_id: string;
  account_id: string;
  subject: string;
  from_name: string | null;
  from_address: string;
  date: string;
  size_bytes: number;
  flags: {
    is_read: boolean;
    is_flagged: boolean;
    is_answered: boolean;
  };
  stored_local: boolean;
  stored_remote: boolean;
  has_attachment: boolean;
}

export async function listMessages(
  folderId: string,
  limit?: number,
  offset?: number,
): Promise<Message[]> {
  return invoke<Message[]>('list_messages', { folderId, limit, offset });
}
