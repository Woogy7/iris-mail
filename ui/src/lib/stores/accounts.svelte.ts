import { listAccounts, type Account } from '$lib/api/accounts';

let accounts = $state<Account[]>([]);
let isLoading = $state(false);

export async function loadAccounts() {
  isLoading = true;
  try {
    accounts = await listAccounts();
  } catch (e) {
    console.error('Failed to load accounts:', e);
  } finally {
    isLoading = false;
  }
}

export function getAccounts(): Account[] {
  return accounts;
}

export function getIsLoading(): boolean {
  return isLoading;
}
