<script lang="ts">
  import { onMount } from 'svelte';
  import TabBar from './lib/components/TabBar.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import { resolvePanels } from './lib/components/panels/registry';
  import { layout, loadConfig } from './lib/stores/config';
  import { alerts, effectivePhosphor } from './lib/stores/theme';
  import { offlineDevices } from './lib/stores/lan';
  import './lib/theme/phosphor.css';

  const main = $derived(resolvePanels([$layout.main])[0]);
  const side = $derived(resolvePanels($layout.side));

  onMount(() => {
    loadConfig();
    return offlineDevices.subscribe((d) => alerts.set(d.length));
  });
</script>

<div class="app-shell" data-phosphor={$effectivePhosphor}>
  <TabBar />
  <div class="panel-grid" class:single={side.length === 0}>
    {#if main}
      {@const Main = main.component}
      <Main />
    {/if}
    {#if side.length > 0}
      <div class="side-column" style:grid-template-rows="repeat({side.length}, 1fr)">
        {#each side as panel (panel.id)}
          {@const Panel = panel.component}
          <Panel />
        {/each}
      </div>
    {/if}
  </div>
  <StatusBar />
</div>
