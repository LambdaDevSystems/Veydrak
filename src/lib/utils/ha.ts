import { pushRing } from '../stores/sysmon';

export interface HaEntity {
  entity_id: string;
  state: string;
  unit: string | null;
  friendly_name: string | null;
  last_updated: string | null;
}

export interface HaRow {
  entity: HaEntity;
  /** Recent numeric values, oldest first; non-numeric states (on/off, unavailable) are not recorded. */
  history: number[];
}

/** HA states are strings; only finite numbers are plottable. */
export function numericState(state: string): number | null {
  if (state.trim() === '') return null;
  const n = Number(state);
  return Number.isFinite(n) ? n : null;
}

export function isUnavailable(state: string): boolean {
  return state === 'unavailable' || state === 'unknown';
}

/** Merge received entity states into the row map (immutably), extending numeric history. */
export function mergeEntities(rows: Record<string, HaRow>, list: HaEntity[]): Record<string, HaRow> {
  const next = { ...rows };
  for (const e of list) {
    const prev = next[e.entity_id];
    const n = numericState(e.state);
    const base = prev?.history ?? [];
    // Keep the friendly name / unit we already know if an update omits them.
    const entity: HaEntity = {
      ...e,
      unit: e.unit ?? prev?.entity.unit ?? null,
      friendly_name: e.friendly_name ?? prev?.entity.friendly_name ?? null,
    };
    next[e.entity_id] = { entity, history: n === null ? base : pushRing(base, n) };
  }
  return next;
}
