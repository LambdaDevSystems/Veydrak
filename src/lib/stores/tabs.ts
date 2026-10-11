import { get, writable } from 'svelte/store';

export interface Tab {
  id: string;
  title: string;
}

let counter = 0;
const newTab = (): Tab => {
  counter += 1;
  return { id: `pty-${counter}`, title: `shell ${counter}` };
};

const first = newTab();
export const tabs = writable<Tab[]>([first]);
export const activeTabId = writable<string>(first.id);

export function addTab(): void {
  const t = newTab();
  tabs.update((all) => [...all, t]);
  activeTabId.set(t.id);
}

export function closeTab(id: string): void {
  const all = get(tabs);
  if (all.length <= 1) return;
  const idx = all.findIndex((t) => t.id === id);
  const next = all.filter((t) => t.id !== id);
  tabs.set(next);
  if (get(activeTabId) === id) {
    activeTabId.set(next[Math.max(0, idx - 1)].id);
  }
}
