import { invoke } from '@tauri-apps/api/core';

export interface Account {
  id: string;
  provider: 'M365' | 'Gmail' | 'ImapGeneric';
  display_name: string;
  email: string;
  accent_colour: string;
  is_enabled: boolean;
}

export async function listAccounts(): Promise<Account[]> {
  return invoke<Account[]>('list_accounts');
}
