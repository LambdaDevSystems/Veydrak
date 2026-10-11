import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import { basePhosphor, type Phosphor } from './theme';

export interface Layout {
  main: string;
  side: string[];
}

export interface LoadedConfig {
  config: { theme: string; check_updates: boolean; layout: Layout };
  warning: string | null;
}

export const DEFAULT_LAYOUT: Layout = { main: 'terminal', side: ['sysmon', 'lan'] };

export const layout = writable<Layout>(DEFAULT_LAYOUT);
export const configWarning = writable<string | null>(null);

/** Load config.json (already validated by the backend) and apply it to the stores. Resolves to `check_updates`. */
export async function loadConfig(): Promise<boolean> {
  try {
    const { config, warning } = await invoke<LoadedConfig>('get_config');
    layout.set(config.layout);
    configWarning.set(warning);
    if (config.theme === 'green' || config.theme === 'amber') basePhosphor.set(config.theme as Phosphor);
    return config.check_updates;
  } catch (e) {
    configWarning.set(`could not load config: ${e}`);
    return false;
  }
}

export async function persistTheme(theme: Phosphor): Promise<void> {
  try {
    await invoke('set_theme', { theme });
  } catch (e) {
    configWarning.set(String(e));
  }
}
