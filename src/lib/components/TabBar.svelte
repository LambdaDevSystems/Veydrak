<script lang="ts">
  import { tabs, activeTabId, addTab, closeTab } from '../stores/tabs';
</script>

<div class="tabbar">
  {#each $tabs as tab (tab.id)}
    <button class="tab" class:active={tab.id === $activeTabId} onclick={() => activeTabId.set(tab.id)}>
      {tab.title}
      {#if $tabs.length > 1}
        <span
          class="x"
          role="button"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); closeTab(tab.id); }}
          onkeydown={(e) => e.key === 'Enter' && closeTab(tab.id)}>×</span>
      {/if}
    </button>
  {/each}
  <button class="tab add" onclick={addTab} aria-label="New tab">+</button>
</div>

<style>
  .tabbar { display: flex; gap: 4px; padding: 6px 8px 0; border-bottom: 1px solid var(--border); font-family: var(--font-display); }
  .tab { background: none; color: var(--fg-dim); border: 1px solid var(--border); border-bottom: none; padding: 4px 12px; cursor: pointer; font: inherit; }
  .tab.active { color: var(--fg); background: var(--panel-bg); }
  .x { margin-left: 8px; opacity: 0.7; }
</style>
