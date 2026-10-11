<script lang="ts">
  import { onMount } from 'svelte';
  import { haEntities } from '../../stores/config';
  import { haRows, haStatus, saveHaToken, startHa } from '../../stores/ha';
  import { sanitizeDeviceName } from '../../utils/format';
  import { isUnavailable, numericState } from '../../utils/ha';
  import { autoRange, sparklinePoints } from '../../utils/sparkline';

  let token = $state('');
  let tokenError = $state<string | null>(null);
  let saving = $state(false);

  onMount(() => {
    const p = startHa();
    return () => { p.then((un) => un()); };
  });

  const STATUS_TEXT: Record<string, string> = {
    no_config: 'NOT CONFIGURED',
    no_token: 'TOKEN REQUIRED',
    connecting: 'CONNECTING…',
    online: 'ONLINE',
    offline: 'OFFLINE, RETRYING',
    auth_failed: 'TOKEN REJECTED',
  };

  const needsToken = $derived($haStatus.status === 'no_token' || $haStatus.status === 'auth_failed');
  const stale = $derived($haStatus.status !== 'online');

  async function submit(e: Event) {
    e.preventDefault();
    saving = true;
    tokenError = await saveHaToken(token);
    if (!tokenError) token = '';
    saving = false;
  }

  function display(id: string): { label: string; value: string; unit: string; dim: boolean; numeric: boolean } {
    const cfg = $haEntities.find((e) => e.id === id);
    const row = $haRows[id];
    const label = sanitizeDeviceName(cfg?.label ?? row?.entity.friendly_name ?? id);
    if (!row) return { label, value: '—', unit: '', dim: true, numeric: false };
    const raw = row.entity.state;
    const n = numericState(raw);
    return {
      label,
      value: n !== null ? String(Math.round(n * 100) / 100) : sanitizeDeviceName(raw, 24),
      unit: n !== null && row.entity.unit ? sanitizeDeviceName(row.entity.unit, 8) : '',
      dim: isUnavailable(raw),
      numeric: n !== null,
    };
  }
</script>

<section class="panel">
  <h2>Home Assistant <span class="status" class:ok={$haStatus.status === 'online'}>{STATUS_TEXT[$haStatus.status]}</span></h2>

  {#if $haStatus.message}
    <div class="dim msg">{sanitizeDeviceName($haStatus.message, 120)}</div>
  {/if}

  {#if needsToken}
    <form onsubmit={submit}>
      <input
        type="password"
        autocomplete="off"
        spellcheck="false"
        placeholder="long-lived access token"
        bind:value={token}
      />
      <button disabled={saving || token.trim() === ''}>save</button>
    </form>
    {#if tokenError}<div class="err">{sanitizeDeviceName(tokenError, 160)}</div>{/if}
  {/if}

  {#if $haEntities.length === 0}
    <div class="dim">add entities under "homeassistant" in config.json</div>
  {/if}

  <ul class:stale>
    {#each $haEntities as cfg (cfg.id)}
      {@const d = display(cfg.id)}
      {@const hist = $haRows[cfg.id]?.history ?? []}
      <li class:dim={d.dim}>
        <span class="label">{d.label}</span>
        <span class="value">{d.value}{#if d.unit}<small> {d.unit}</small>{/if}</span>
        {#if d.numeric && hist.length > 1}
          {@const r = autoRange(hist)}
          <svg viewBox="0 0 100 30" preserveAspectRatio="none"><polyline points={sparklinePoints(hist, r.min, r.max)} /></svg>
        {/if}
      </li>
    {/each}
  </ul>
</section>

<style>
  .status { margin-left: 8px; color: var(--fg-dim); }
  .status.ok { color: var(--fg); }
  ul { list-style: none; margin: 0; padding: 0; }
  ul.stale { opacity: 0.55; }
  li { display: grid; grid-template-columns: 1fr auto; gap: 2px 8px; padding: 4px 0; border-bottom: 1px solid var(--fg-dim); }
  li.dim { color: var(--fg-dim); }
  .value { text-align: right; }
  small { color: var(--fg-dim); }
  svg { grid-column: 1 / -1; width: 100%; height: 24px; }
  polyline { fill: none; stroke: var(--fg); stroke-width: 1; vector-effect: non-scaling-stroke; }
  .dim { color: var(--fg-dim); }
  .msg { margin-bottom: 4px; }
  .err { color: #ff4d4d; margin: 4px 0; }
  form { display: flex; gap: 4px; margin-bottom: 6px; }
  input { flex: 1; min-width: 0; background: none; color: var(--fg); border: 1px solid var(--border); font: inherit; padding: 2px 4px; }
  button { background: none; color: var(--fg); border: 1px solid var(--border); font: inherit; cursor: pointer; }
  button:disabled { opacity: 0.5; cursor: default; }
</style>
