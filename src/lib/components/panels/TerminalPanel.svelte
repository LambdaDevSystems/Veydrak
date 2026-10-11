<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { tabs, activeTabId } from '../../stores/tabs';
  import { effectivePhosphor } from '../../stores/theme';
  import { phosphorToXtermTheme } from '../../utils/xterm-theme';

  interface Session { term: Terminal; fit: FitAddon; el: HTMLDivElement; unlisten: UnlistenFn }

  let host: HTMLDivElement;
  const sessions = new Map<string, Session>();
  let observer: ResizeObserver;

  async function open(id: string) {
    const el = document.createElement('div');
    el.className = 'term-host';
    host.appendChild(el);
    const term = new Terminal({
      fontFamily: "'IBM Plex Mono', monospace",
      fontSize: 14,
      cursorBlink: true,
      allowTransparency: true,
      theme: phosphorToXtermTheme($effectivePhosphor),
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();
    const unlisten = await listen<string>(`pty-output:${id}`, (e) => term.write(e.payload));
    term.onData((data) => invoke('write_pty', { id, data }));
    term.onResize(({ rows, cols }) => invoke('resize_pty', { id, rows, cols }));
    await invoke('create_pty', { id, rows: term.rows, cols: term.cols });
    sessions.set(id, { term, fit, el, unlisten });
  }

  async function close(id: string) {
    const s = sessions.get(id);
    if (!s) return;
    s.unlisten();
    s.term.dispose();
    s.el.remove();
    sessions.delete(id);
    await invoke('close_pty', { id });
  }

  // Reconcile sessions with the tab list and show only the active one.
  $effect(() => {
    const ids = new Set($tabs.map((t) => t.id));
    for (const t of $tabs) if (!sessions.has(t.id)) open(t.id);
    for (const id of [...sessions.keys()]) if (!ids.has(id)) close(id);
  });

  $effect(() => {
    const active = $activeTabId;
    for (const [id, s] of sessions) {
      s.el.style.display = id === active ? 'block' : 'none';
      if (id === active) { s.fit.fit(); s.term.focus(); }
    }
  });

  $effect(() => {
    const theme = phosphorToXtermTheme($effectivePhosphor);
    for (const s of sessions.values()) s.term.options.theme = theme;
  });

  onMount(() => {
    observer = new ResizeObserver(() => sessions.get($activeTabId)?.fit.fit());
    observer.observe(host);
  });

  onDestroy(() => {
    observer?.disconnect();
    for (const id of [...sessions.keys()]) close(id);
  });
</script>

<section class="panel terminal">
  <div class="host" bind:this={host}></div>
</section>

<style>
  .terminal { padding: 4px; overflow: hidden; }
  .host { height: 100%; }
  .host :global(.term-host) { height: 100%; }
</style>
