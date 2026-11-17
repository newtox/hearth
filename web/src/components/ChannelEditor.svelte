<script lang="ts">
  import { t } from '../lib/i18n.svelte';
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import { CHANNEL_PERMISSIONS } from '../lib/perms';
  import { app, attempt, can, guild, P, type Channel } from '../lib/store.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { modal: live }: { modal: { channel: Channel | null; parent?: string | null; type?: 'text' | 'category' | 'voice' } } = $props();
  const modal = untrack(() => live);

  const g = $derived(guild()!);
  const existing = $derived(modal.channel ? (g.channels.find((c) => c.id === modal.channel!.id) ?? modal.channel) : null);
  const kind = modal.channel?.kind ?? modal.type ?? 'text';
  const voiceBits = [P.VIEW_CHANNEL, P.CONNECT, P.SPEAK, P.STREAM];
  const rules = CHANNEL_PERMISSIONS.filter((p) => (kind === 'voice' ? voiceBits.includes(p.bit) : p.bit === P.VIEW_CHANNEL || !voiceBits.includes(p.bit)));
  const categories = $derived(g.channels.filter((c) => c.kind === 'category'));

  let name = $state(modal.channel?.name ?? '');
  let topic = $state(modal.channel?.topic ?? '');
  let parent = $state<string>(modal.channel?.parent_id ?? modal.parent ?? '');

  function close() {
    app.modal = null;
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    const body = { name, topic, kind, parent_id: kind !== 'category' ? parent || null : null };
    const ok = existing ? await attempt(api('PATCH', `/channels/${existing.id}`, body)) : await attempt(api('POST', `/guilds/${g.id}/channels`, body));
    if (ok) close();
  }

  async function remove() {
    if (existing && confirm(t('„{name}“ mit allen Nachrichten löschen?', { name: existing.name }))) {
      await attempt(api('DELETE', `/channels/${existing.id}`));
      close();
    }
  }

  let extra = $state<string[]>([]);
  const targets = $derived.by(() => {
    const ids = new Set([...(existing?.overwrites.filter((o) => o.kind === 'member').map((o) => o.target_id) ?? []), ...extra]);
    return [
      ...g.roles.map((r) => ({ id: r.id, kind: 'role' as const, name: r.name, color: r.color })),
      ...g.members.filter((m) => ids.has(m.id)).map((m) => ({ id: m.id, kind: 'member' as const, name: m.nickname || m.display_name, color: null })),
    ];
  });
  const addable = $derived(g.members.filter((m) => !targets.some((x) => x.id === m.id)));

  function rule(roleId: string, bit: number) {
    const o = existing?.overwrites.find((x) => x.target_id === roleId);
    return o && o.allow & bit ? 1 : o && o.deny & bit ? -1 : 0;
  }

  async function setState(roleId: string, bit: number, value: number, kind: 'role' | 'member' = 'role') {
    if (!existing) return;
    const o = existing.overwrites.find((x) => x.target_id === roleId);
    let allow = (o?.allow ?? 0) & ~bit;
    let deny = (o?.deny ?? 0) & ~bit;
    if (value === 1) allow |= bit;
    if (value === -1) deny |= bit;
    if (!allow && !deny) await attempt(api('DELETE', `/channels/${existing.id}/overwrites/${roleId}`));
    else await attempt(api('PUT', `/channels/${existing.id}/overwrites/${roleId}`, { kind, allow, deny }));
  }
</script>

<Modal title={t((kind === 'category' ? 'Kategorie' : kind === 'voice' ? 'Sprachkanal' : 'Kanal') + (existing ? ' bearbeiten' : ' anlegen'))} onclose={close} wide={!!existing && kind !== 'category'}>
  <div class="stack">
    <form class="stack" onsubmit={save}>
      <label class="field">
        <span>{t('Name')}</span>
        <input class="input" bind:value={name} maxlength="60" required />
      </label>
      {#if kind === 'text'}
        <label class="field">
          <span>{t('Thema')}</span>
          <input class="input" bind:value={topic} maxlength="300" placeholder={t('Worum geht es hier?')} />
        </label>
      {/if}
      {#if kind !== 'category'}
        <label class="field">
          <span>{t('Kategorie')}</span>
          <select class="input" bind:value={parent}>
            <option value="">{t('Keine')}</option>
            {#each categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          </select>
        </label>
      {/if}
      <div class="row">
        <button class="btn primary">{existing ? t('Änderungen speichern') : t('Anlegen')}</button>
        {#if existing}<button type="button" class="btn danger" onclick={remove}>{t('Löschen')}</button>{/if}
      </div>
    </form>

    {#if existing && kind !== 'category' && can(P.MANAGE_ROLES)}
      <div>
        <h3>{t('Kanalregeln')}</h3>
        <p class="muted">{t('Pro Rolle oder Mitglied erlauben, verbieten oder die Server-Einstellung übernehmen. Regeln für ein Mitglied gehen vor. Änderungen gelten sofort.')}</p>
      </div>
      <div class="rules">
        <table>
          <thead>
            <tr>
              <th>{t('Rolle oder Mitglied')}</th>
              {#each rules as p}<th>{t(p.name)}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each targets as r (r.id)}
              <tr>
                <td>{#if r.kind === 'role'}<span class="swatch" style:background={r.color ?? 'var(--muted)'}></span>{:else}<span class="person"><Icon name="users" size={13} /></span>{/if}{r.name}</td>
                {#each rules as p}
                  {@const s = rule(r.id, p.bit)}
                  <td>
                    <div class="tri" role="group" aria-label={t('{perm} für {role}', { perm: t(p.name), role: r.name })}>
                      <button class:on={s === -1} class="no" onclick={() => setState(r.id, p.bit, -1, r.kind)} aria-pressed={s === -1}>{t('Nein')}</button>
                      <button class:on={s === 0} onclick={() => setState(r.id, p.bit, 0, r.kind)} aria-pressed={s === 0}>{t('Erben')}</button>
                      <button class:on={s === 1} class="yes" onclick={() => setState(r.id, p.bit, 1, r.kind)} aria-pressed={s === 1}>{t('Ja')}</button>
                    </div>
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if addable.length}
        <label class="field">
          <span>{t('Regel für ein einzelnes Mitglied hinzufügen')}</span>
          <select class="input" value="" onchange={(e) => ((extra = [...extra, e.currentTarget.value]), (e.currentTarget.value = ''))}>
            <option value="">{t('Mitglied auswählen')}</option>
            {#each addable as m (m.id)}<option value={m.id}>{m.nickname || m.display_name}</option>{/each}
          </select>
        </label>
      {/if}
    {/if}
  </div>
</Modal>

<style>
  .rules {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  th,
  td {
    text-align: left;
    padding: 6px 10px 6px 0;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  th {
    font-size: 0.85em;
    color: var(--muted);
  }
  .person {
    display: inline-flex;
    margin-right: 6px;
    vertical-align: -2px;
    color: var(--muted);
  }
  .swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    margin-right: 8px;
  }
  .tri {
    display: inline-flex;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    overflow: hidden;
  }
  .tri button {
    padding: 2px 9px;
    font-size: 0.85em;
    color: var(--muted);
  }
  .tri button.on {
    background: var(--panel-2);
    color: var(--ink);
    font-weight: 700;
  }
  .tri button.on.yes {
    background: var(--ok);
    color: #06200f;
  }
  .tri button.on.no {
    background: var(--danger);
    color: #2a0006;
  }
  :global([data-theme='hell']) .tri button.on.yes,
  :global([data-theme='hell']) .tri button.on.no,
  :global([data-theme='pergament']) .tri button.on.yes,
  :global([data-theme='pergament']) .tri button.on.no {
    color: #fff;
  }
</style>
