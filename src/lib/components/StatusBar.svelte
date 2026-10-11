<script lang="ts">
  import { basePhosphor, effectivePhosphor, type Phosphor } from '../stores/theme';
  import { offlineDevices } from '../stores/lan';

  const palettes: Phosphor[] = ['green', 'amber'];
  const next = () => basePhosphor.update((p) => palettes[(palettes.indexOf(p) + 1) % palettes.length]);
</script>

<div class="status">
  <span>SENTINEL</span>
  <span>
    {#if $offlineDevices.length > 0}
      ALERT: {$offlineDevices.length} pinned device(s) offline
    {:else}
      ALL SYSTEMS NOMINAL
    {/if}
  </span>
  <button onclick={next}>phosphor: {$effectivePhosphor}</button>
</div>

<style>
  .status { display: flex; justify-content: space-between; align-items: center; padding: 4px 12px; border-top: 1px solid var(--border); font-family: var(--font-display); color: var(--fg-dim); }
  button { background: none; border: 1px solid var(--border); color: var(--fg); font: inherit; cursor: pointer; }
</style>
