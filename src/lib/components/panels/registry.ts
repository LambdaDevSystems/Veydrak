import type { Component } from 'svelte';
import TerminalPanel from './TerminalPanel.svelte';
import SystemMonitorPanel from './SystemMonitorPanel.svelte';
import LanRosterPanel from './LanRosterPanel.svelte';
import { resolvePanels as resolve } from './resolve';

/** Panel ids usable in `layout.main` / `layout.side` of config.json. Keep in sync with config.rs. */
export const panelRegistry: Record<string, Component> = {
  terminal: TerminalPanel,
  sysmon: SystemMonitorPanel,
  lan: LanRosterPanel,
};

export const resolvePanels = (ids: string[]) => resolve(ids, panelRegistry);
