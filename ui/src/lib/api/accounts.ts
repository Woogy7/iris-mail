import { invoke } from '@tauri-apps/api/core';

export type Provider = 'M365' | 'Gmail' | 'ImapGeneric';

export type AccentColour = 'Red' | 'Peach' | 'Yellow' | 'Green' | 'Sapphire' | 'Mauve' | 'Lavender';

export interface SyncPreferences {
  initial_sync_days: number;
  rate_limit_per_minute: number;
  poll_interval_secs: number;
  synced_tier_bytes: number;
  is_archive_enabled: boolean;
}

export interface Account {
  id: string;
  display_name: string;
  email_address: string;
  provider: Provider;
  keychain_ref: string;
  sync_preferences: SyncPreferences;
  accent_colour: AccentColour;
  created_at: string;
  updated_at: string;
}

export async function listAccounts(): Promise<Account[]> {
  return invoke<Account[]>('list_accounts');
}

export async function getAccount(accountId: string): Promise<Account> {
  return invoke<Account>('get_account', { accountId });
}

export async function addM365Account(emailAddress: string, displayName: string): Promise<Account> {
  return invoke<Account>('add_m365_account', { emailAddress, displayName });
}

export async function addImapAccount(params: {
  emailAddress: string;
  displayName: string;
  password: string;
}): Promise<Account> {
  return invoke<Account>('add_imap_account', params);
}

export async function removeAccount(accountId: string): Promise<void> {
  return invoke<void>('remove_account', { accountId });
}
