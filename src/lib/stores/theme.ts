import { derived, writable } from 'svelte/store';

export type Phosphor = 'green' | 'amber' | 'red';

/** User-selected palette. */
export const basePhosphor = writable<Phosphor>('green');

/** Number of active alerts (e.g. offline pinned devices). */
export const alerts = writable<number>(0);

/** Palette actually rendered: any active alert overrides the base palette with red. */
export const effectivePhosphor = derived(
  [basePhosphor, alerts],
  ([$base, $alerts]): Phosphor => ($alerts > 0 ? 'red' : $base),
);
