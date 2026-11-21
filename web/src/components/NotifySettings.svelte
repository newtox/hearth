<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { w } from '../lib/theme.svelte';
  import { inheritedLevel, notifyPref, setNotify, type NotifyLevel } from '../lib/store.svelte';

  let { id, kind }: { id: string; kind: 'guild' | 'channel' | 'category' } = $props();

  const pref = $derived(notifyPref(id));
  const LEVELS: { id: NotifyLevel; name: string }[] = [
    { id: 'all', name: 'notify_all' },
    { id: 'mentions', name: 'notify_mentions' },
    { id: 'none', name: 'notify_none' },
  ];
  const options = $derived<{ id: NotifyLevel | null; name: string }[]>([
    ...(kind === 'guild' ? [] : [{ id: null, name: t('notify_default', { level: t(LEVELS.find((l) => l.id === inheritedLevel(id))!.name) }) }]),
    ...LEVELS.map((l) => ({ id: l.id, name: t(l.name) })),
  ]);
  const chosen = $derived(kind === 'guild' ? (pref.level ?? 'mentions') : pref.level);
  const muteLabel = $derived(t(kind === 'guild' ? 'notify_mute_server' : kind === 'category' ? 'notify_mute_category' : 'notify_mute_channel', { server: w('server') }));
</script>

<div class="stack">
  <p class="muted">{t('notify_page_hint')}</p>
  <label class="switch row">
    <span class="grow"><strong>{muteLabel}</strong><small class="muted">{t('notify_muted_hint')}</small></span>
    <input type="checkbox" checked={pref.muted} onchange={(e) => setNotify(id, { muted: e.currentTarget.checked })} />
  </label>
  <fieldset class="levels" class:dim={pref.muted}>
    <legend>{t('notify_level_title')}</legend>
    {#each options as o (o.id ?? 'default')}
      <label class="opt row">
        <input type="radio" name="notify-{id}" checked={chosen === o.id} onchange={() => setNotify(id, { level: o.id })} />
        <span>{o.name}</span>
      </label>
    {/each}
  </fieldset>
  {#if kind === 'guild'}
    <label class="switch row">
      <span class="grow">{t('notify_suppress_everyone')}</span>
      <input type="checkbox" checked={pref.suppress_everyone} onchange={(e) => setNotify(id, { suppress_everyone: e.currentTarget.checked })} />
    </label>
    <label class="switch row">
      <span class="grow">{t('notify_suppress_roles')}</span>
      <input type="checkbox" checked={pref.suppress_roles} onchange={(e) => setNotify(id, { suppress_roles: e.currentTarget.checked })} />
    </label>
  {/if}
</div>

<style>
  .switch,
  .levels {
    padding: 12px 14px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    border: 0;
    margin: 0;
  }
  .switch {
    cursor: pointer;
    gap: 14px;
  }
  .switch strong,
  .switch small {
    display: block;
  }
  .switch input,
  .opt input {
    flex: none;
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
  }
  .levels {
    display: grid;
    gap: 2px;
  }
  .levels.dim {
    opacity: 0.55;
  }
  legend {
    float: left;
    width: 100%;
    margin-bottom: 6px;
    font-size: 0.85em;
    font-weight: 700;
    color: var(--muted);
  }
  .opt {
    padding: 8px 4px;
    cursor: pointer;
  }
</style>
