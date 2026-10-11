import { derived, writable } from 'svelte/store';

export type DeviceStatus = 'online' | 'offline' | 'unknown';

export interface Device {
  ip: string;
  name: string;
  status: DeviceStatus;
  pinned: boolean;
}

export const roster = writable<Device[]>([]);

/** Pinned devices that are confirmed offline; drives the red alert palette. */
export const offlineDevices = derived(roster, ($r) =>
  $r.filter((d) => d.pinned && d.status === 'offline'),
);
