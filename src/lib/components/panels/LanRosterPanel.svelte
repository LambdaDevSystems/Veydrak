<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { roster, type Device } from '../../stores/lan';
  import { sanitizeDeviceName } from '../../utils/format';

  onMount(() => {
    invoke<Device[]>('get_roster').then((r) => roster.set(r));
    const p = listen<Device[]>('lan-roster', (e) => roster.set(e.payload));
    return () => { p.then((un) => un()); };
  });

  const togglePin = (ip: string) => invoke('toggle_pin', { ip });
</script>

<section class="panel">
  <h2>LAN</h2>
  {#if $roster.length === 0}
    <div class="dim">scanning…</div>
  {/if}
  <ul>
    {#each $roster as d (d.ip)}
      <li class={d.status}>
        <button class="pin" onclick={() => togglePin(d.ip)} aria-label="Toggle pin">{d.pinned ? '★' : '☆'}</button>
        <span class="dot"></span>
        <span class="name">{sanitizeDeviceName(d.name)}</span>
        <span class="ip dim">{d.ip}</span>
      </li>
    {/each}
  </ul>
</section>

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; gap: 8px; align-items: center; padding: 2px 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--fg); }
  li.offline .dot { background: #ff4d4d; }
  li.unknown .dot { background: var(--fg-dim); }
  .ip { margin-left: auto; }
  .dim { color: var(--fg-dim); }
  .pin { background: none; border: none; color: var(--accent); cursor: pointer; padding: 0; }
</style>
