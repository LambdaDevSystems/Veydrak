import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';
import { mergeEntities, type HaEntity, type HaRow } from '../utils/ha';

export type HaStatusName = 'no_config' | 'no_token' | 'connecting' | 'online' | 'offline' | 'auth_failed';

export interface HaStatus {
  status: HaStatusName;
  message: string | null;
}

export const haStatus = writable<HaStatus>({ status: 'no_config', message: null });
export const haRows = writable<Record<string, HaRow>>({});

/** Load the current snapshot and subscribe to live updates. Returns an unsubscribe function. */
export async function startHa(): Promise<() => void> {
  const unStatus = await listen<HaStatus>('ha-status', (e) => haStatus.set(e.payload));
  const unEntities = await listen<HaEntity[]>('ha-entities', (e) =>
    haRows.update((rows) => mergeEntities(rows, e.payload)),
  );
  try {
    const snap = await invoke<{ status: HaStatus; entities: HaEntity[] }>('ha_snapshot');
    haStatus.set(snap.status);
    haRows.update((rows) => mergeEntities(rows, snap.entities));
  } catch (e) {
    haStatus.set({ status: 'offline', message: String(e) });
  }
  return () => {
    unStatus();
    unEntities();
  };
}

/** Sends the token to the backend (OS keyring). It is never read back. */
export async function saveHaToken(token: string): Promise<string | null> {
  try {
    await invoke('set_ha_token', { token });
    return null;
  } catch (e) {
    return String(e);
  }
}
