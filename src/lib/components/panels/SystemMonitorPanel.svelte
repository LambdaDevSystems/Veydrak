<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { cpuHistory, memHistory, memTotal, battery, ingestSample, type SysSample } from '../../stores/sysmon';
  import { formatBytes } from '../../utils/format';
  import { sparklinePoints } from '../../utils/sparkline';

  onMount(() => {
    const p = listen<SysSample>('sysmon', (e) => ingestSample(e.payload));
    return () => { p.then((un) => un()); };
  });

  const cpu = $derived($cpuHistory.at(-1) ?? 0);
  const mem = $derived($memHistory.at(-1) ?? 0);
</script>

<section class="panel">
  <h2>System</h2>
  <div>CPU {cpu.toFixed(0)}%</div>
  <svg viewBox="0 0 100 30" preserveAspectRatio="none"><polyline points={sparklinePoints($cpuHistory, 0, 100)} /></svg>
  <div>MEM {formatBytes(mem)} / {formatBytes($memTotal)}</div>
  <svg viewBox="0 0 100 30" preserveAspectRatio="none"><polyline points={sparklinePoints($memHistory, 0, $memTotal || 1)} /></svg>
  {#if $battery}
    <div>BAT {$battery.percent}%{$battery.charging ? ' (charging)' : ''}</div>
  {/if}
</section>

<style>
  svg { width: 100%; height: 48px; margin: 4px 0 10px; border-bottom: 1px solid var(--fg-dim); }
  polyline { fill: none; stroke: var(--fg); stroke-width: 1; vector-effect: non-scaling-stroke; }
</style>
