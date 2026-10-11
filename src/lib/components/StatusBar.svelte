<script lang="ts">
  import { basePhosphor, effectivePhosphor, type Phosphor } from '../stores/theme';
  import { offlineDevices } from '../stores/lan';
  import { configWarning, persistTheme } from '../stores/config';
  import { get } from 'svelte/store';
  import { update, updatePhase, updateProgress, installUpdate } from '../stores/update';

  const palettes: Phosphor[] = ['green', 'amber'];
  const next = () => {
    const p = get(basePhosphor);
    const n = palettes[(palettes.indexOf(p) + 1) % palettes.length];
    basePhosphor.set(n);
    persistTheme(n);
  };
</script>

<div class="status">
  <span>SENTINEL</span>
  <span>
    {#if $configWarning}
      CONFIG: {$configWarning}
    {:else if $offlineDevices.length > 0}
      ALERT: {$offlineDevices.length} pinned device(s) offline
    {:else}
      ALL SYSTEMS NOMINAL
    {/if}
  </span>
  {#if $update && $updatePhase === 'available'}
    <button onclick={installUpdate}>UPDATE v{$update.version} available: install &amp; restart</button>
  {:else if $updatePhase === 'installing'}
    <span>UPDATING {$updateProgress === null ? '…' : `${Math.round($updateProgress * 100)}%`}</span>
  {:else if $updatePhase === 'error'}
    <span>UPDATE FAILED</span>
  {/if}
  <button onclick={next}>phosphor: {$effectivePhosphor}</button>
</div>

<style>
  .status { display: flex; justify-content: space-between; align-items: center; padding: 4px 12px; border-top: 1px solid var(--border); font-family: var(--font-display); color: var(--fg-dim); }
  button { background: none; border: 1px solid var(--border); color: var(--fg); font: inherit; cursor: pointer; }
</style>
