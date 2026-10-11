import { writable } from 'svelte/store';

export const HISTORY_LEN = 60;

export interface Battery {
  percent: number;
  charging: boolean;
}

export interface SysSample {
  cpu: number; // percent, 0-100
  mem_used: number; // bytes
  mem_total: number; // bytes
  battery: Battery | null;
}

/** Fixed-size ring: appends and drops the oldest entry once full. */
export function pushRing<T>(buf: T[], value: T, max = HISTORY_LEN): T[] {
  const next = buf.length >= max ? buf.slice(buf.length - max + 1) : buf.slice();
  next.push(value);
  return next;
}

export const cpuHistory = writable<number[]>([]);
export const memHistory = writable<number[]>([]);
export const memTotal = writable<number>(0);
export const battery = writable<Battery | null>(null);

export function ingestSample(s: SysSample): void {
  cpuHistory.update((h) => pushRing(h, s.cpu));
  memHistory.update((h) => pushRing(h, s.mem_used));
  memTotal.set(s.mem_total);
  battery.set(s.battery);
}
