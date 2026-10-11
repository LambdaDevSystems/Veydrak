import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

export interface UpdateInfo {
  version: string;
  notes: string | null;
}

export type UpdatePhase = 'idle' | 'available' | 'installing' | 'error';

export const update = writable<UpdateInfo | null>(null);
export const updatePhase = writable<UpdatePhase>('idle');
/** 0..1 while installing; null when the total size is unknown. */
export const updateProgress = writable<number | null>(null);

/** Silent check: a failed or offline check just leaves the UI as it was. */
export async function checkForUpdate(): Promise<void> {
  try {
    const info = await invoke<UpdateInfo | null>('check_update');
    update.set(info);
    updatePhase.set(info ? 'available' : 'idle');
  } catch (e) {
    console.warn('update check failed:', e);
  }
}

/** Only ever called from an explicit user click. The backend restarts the app when done. */
export async function installUpdate(): Promise<void> {
  updatePhase.set('installing');
  updateProgress.set(null);
  const un = await listen<{ downloaded: number; total: number | null }>('update-progress', (e) => {
    const { downloaded, total } = e.payload;
    updateProgress.set(total ? Math.min(downloaded / total, 1) : null);
  });
  try {
    await invoke('install_update');
  } catch (e) {
    console.warn('update install failed:', e);
    updatePhase.set('error');
  } finally {
    un();
  }
}
