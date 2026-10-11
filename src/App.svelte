<script lang="ts">
  import { onMount } from 'svelte';
  import TabBar from './lib/components/TabBar.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import TerminalPanel from './lib/components/panels/TerminalPanel.svelte';
  import SystemMonitorPanel from './lib/components/panels/SystemMonitorPanel.svelte';
  import LanRosterPanel from './lib/components/panels/LanRosterPanel.svelte';
  import { alerts, effectivePhosphor } from './lib/stores/theme';
  import { offlineDevices } from './lib/stores/lan';
  import './lib/theme/phosphor.css';

  onMount(() => offlineDevices.subscribe((d) => alerts.set(d.length)));
</script>

<div class="app-shell" data-phosphor={$effectivePhosphor}>
  <TabBar />
  <div class="panel-grid">
    <TerminalPanel />
    <div class="side-column">
      <SystemMonitorPanel />
      <LanRosterPanel />
    </div>
  </div>
  <StatusBar />
</div>
