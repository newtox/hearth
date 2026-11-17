<script lang="ts">
  import { ask, askText } from '../lib/dialog.svelte';
  import { auditDetail, locale, serverError, t } from '../lib/i18n.svelte';
  import { untrack } from 'svelte';
  import { api, asset, base, getToken, upload } from '../lib/api';
  import { PERMISSIONS, has } from '../lib/perms';
  import { app, attempt, can, canModerate, guild, toast, topPosition, P, type Role } from '../lib/store.svelte';
  import { w } from '../lib/theme.svelte';
  import { docs } from '../lib/docs';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import SettingsPage from './SettingsPage.svelte';

  let { tab: initial = 'overview' }: { tab?: string } = $props();

  const g = $derived(guild()!);
  const owner = $derived(g.owner_id === app.me!.id);
  const myTop = $derived(topPosition(g, app.me!.id));
  const tabs = $derived(
    [
      { id: 'overview', name: t('Übersicht'), group: g.name, show: true },
      { id: 'roles', name: w('roles'), group: g.name, show: can(P.MANAGE_ROLES, g) },
      { id: 'emojis', name: t('Emojis'), group: g.name, show: can(P.MANAGE_EMOJIS, g) },
      { id: 'members', name: w('members'), group: t('Mitgliederverwaltung'), show: true },
      { id: 'invites', name: w('invites'), group: t('Mitgliederverwaltung'), show: can(P.CREATE_INVITE, g) },
      { id: 'bans', name: t('Banns'), group: t('Mitgliederverwaltung'), show: can(P.BAN_MEMBERS, g) },
      { id: 'audit', name: t('Protokoll'), group: t('Moderation'), show: can(P.MANAGE_GUILD, g) },
      { id: 'leave', name: owner ? t('{server} löschen', { server: w('server') }) : t('{server} verlassen', { server: w('server') }), danger: true, show: true, action: () => (owner ? destroy() : leave()) },
    ].filter((x) => x.show),
  );
  let tab = $state(untrack(() => initial));

  let name = $state(guild()!.name);
  let description = $state(guild()!.description);

  async function saveOverview(e: SubmitEvent) {
    e.preventDefault();
    if (await attempt(api('PATCH', `/guilds/${g.id}`, { name, description }))) toast(t('Gespeichert'));
  }

  async function pickIcon(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) await attempt(upload(`/guilds/${g.id}/icon`, file));
  }

  async function leave() {
    if ((await ask({ title: t('„{name}“ verlassen', { name: g.name }), text: t('Du kannst nur mit einer neuen Einladung wieder beitreten.'), confirm: t('Verlassen'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${app.me!.id}`));
  }

  async function destroy() {
    if ((await askText({ title: t('„{name}“ löschen', { name: g.name }), text: t('Alle Kanäle, Nachrichten und Rollen gehen verloren. Das lässt sich nicht rückgängig machen.'), input: { label: t('Zum Bestätigen den Servernamen eingeben'), match: g.name }, confirm: t('Server löschen'), danger: true })) === g.name) await attempt(api('DELETE', `/guilds/${g.id}`));
  }

  let roleId = $state<string | null>(null);
  const role = $derived(g.roles.find((r) => r.id === roleId) ?? null);
  let draft = $state({ name: '', color: '#8888aa', colored: false, hoist: false, permissions: 0 });

  function editable(r: Role) {
    return owner || r.position < myTop;
  }

  function pickRole(r: Role) {
    roleId = r.id;
    draft = { name: r.name, color: r.color ?? '#8888aa', colored: !!r.color, hoist: r.hoist, permissions: r.permissions };
  }

  async function addRole() {
    const r = await attempt(api<Role>('POST', `/guilds/${g.id}/roles`, { name: t('Neue Rolle') }));
    if (r) pickRole(r);
  }

  async function saveRole(e: SubmitEvent) {
    e.preventDefault();
    if (!role) return;
    const body = { name: draft.name, color: draft.colored ? draft.color : '', hoist: draft.hoist, permissions: draft.permissions };
    if (await attempt(api('PATCH', `/guilds/${g.id}/roles/${role.id}`, body))) toast(t('Rolle gespeichert'));
  }

  async function deleteRole() {
    if (role && (await ask({ title: t('Rolle löschen'), text: t('„{name}“ wird gelöscht und allen Mitgliedern entzogen.', { name: role.name }), confirm: t('Löschen'), danger: true }))) {
      await attempt(api('DELETE', `/guilds/${g.id}/roles/${role.id}`));
      roleId = null;
    }
  }

  async function move(r: Role, dir: -1 | 1) {
    const order = g.roles.filter((x) => !x.is_default).map((x) => x.id);
    const i = order.indexOf(r.id);
    const j = i + dir;
    if (j < 0 || j >= order.length) return;
    [order[i], order[j]] = [order[j], order[i]];
    await attempt(api('PUT', `/guilds/${g.id}/roles`, order));
  }

  async function toggleRole(memberId: string, current: string[], id: string) {
    const roles = current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
    await attempt(api('PATCH', `/guilds/${g.id}/members/${memberId}`, { roles }));
  }

  async function nickname(memberId: string, current: string | null) {
    const nick = await askText({ title: t('Spitzname ändern'), input: { value: current ?? '', label: t('Spitzname'), placeholder: '' }, text: t('Leer lassen, um den Spitznamen zu entfernen.'), confirm: t('Speichern') });
    if (nick !== null) await attempt(api('PATCH', `/guilds/${g.id}/members/${memberId}`, { nickname: nick }));
  }

  async function kick(id: string, label: string) {
    if ((await ask({ title: t('{name} entfernen', { name: label }), text: t('{name} kann über eine neue Einladung wieder beitreten.', { name: label }), confirm: t('Entfernen'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${id}`));
  }

  async function ban(id: string, label: string) {
    const reason = await askText({ title: t('{name} bannen', { name: label }), text: t('{name} wird entfernt und kann nicht wieder beitreten, bis der Bann aufgehoben wird.', { name: label }), input: { label: t('Grund (optional)') }, confirm: t('Bannen'), danger: true });
    if (reason !== null) await attempt(api('PUT', `/guilds/${g.id}/bans/${id}`, { reason }));
  }

  async function transfer(id: string, label: string) {
    if ((await ask({ title: t('Besitz übertragen'), text: t('{name} wird neuer Besitzer. Das lässt sich nur vom neuen Besitzer rückgängig machen.', { name: label }), confirm: t('Übertragen'), danger: true }))) await attempt(api('PATCH', `/guilds/${g.id}`, { owner_id: id }));
  }

  let invites = $state<any[]>([]);
  let created = $state<string | null>(null);
  let hours = $state(168);
  let maxUses = $state(0);
  let bans = $state<any[]>([]);

  function link(code: string) {
    return `${location.origin}/#/invite/${code}`;
  }

  async function loadInvites() {
    invites = can(P.MANAGE_GUILD, g) ? ((await attempt(api('GET', `/guilds/${g.id}/invites`))) ?? []) : [];
  }

  async function createInvite() {
    const inv = await attempt(api('POST', `/guilds/${g.id}/invites`, { expires_in_hours: hours, max_uses: maxUses }));
    if (inv) {
      created = inv.code;
      loadInvites();
    }
  }

  async function copy(code: string) {
    try {
      await navigator.clipboard.writeText(link(code));
      toast(t('Link kopiert'));
    } catch {
      toast(link(code));
    }
  }

  async function deleteInvite(code: string) {
    await attempt(api('DELETE', `/guilds/${g.id}/invites/${code}`));
    loadInvites();
  }

  async function loadBans() {
    bans = (await attempt(api('GET', `/guilds/${g.id}/bans`))) ?? [];
  }

  async function unban(id: string) {
    await attempt(api('DELETE', `/guilds/${g.id}/bans/${id}`));
    loadBans();
  }

  let audit = $state<any[]>([]);
  const ACTIONS: Record<string, string> = {
    server_update: 'hat die Servereinstellungen geändert',
    role_create: 'hat eine Rolle angelegt',
    role_update: 'hat eine Rolle geändert',
    role_delete: 'hat eine Rolle gelöscht',
    channel_create: 'hat einen Kanal angelegt',
    channel_update: 'hat einen Kanal geändert',
    channel_delete: 'hat einen Kanal gelöscht',
    channel_rules: 'hat Kanalregeln geändert',
    member_roles: 'hat Rollen geändert von',
    timeout: 'hat einen Timeout vergeben an',
    timeout_end: 'hat den Timeout aufgehoben von',
    kick: 'hat entfernt',
    ban: 'hat gebannt',
    unban: 'hat den Bann aufgehoben von',
    message_delete: 'hat eine Nachricht gelöscht von',
    emoji_create: 'hat ein Emoji hochgeladen',
    sticker_create: 'hat einen Sticker hochgeladen',
    emoji_delete: 'hat ein Emoji oder einen Sticker gelöscht',
  };

  async function loadAudit() {
    audit = (await attempt(api('GET', `/guilds/${g.id}/audit`))) ?? [];
  }

  async function timeout(id: string, label: string) {
    const minutes = await askText({ title: t('Timeout für {name}', { name: label }), text: t('Dauer in Minuten, z. B. 10, 60 oder 1440.'), input: { value: '60', type: 'number', label: t('Minuten') }, confirm: t('Timeout') });
    if (minutes === null) return;
    const n = Number(minutes);
    if (!Number.isFinite(n) || n <= 0) return toast(t('Bitte eine Zahl größer 0 eingeben'));
    await attempt(api('PATCH', `/guilds/${g.id}/members/${id}`, { timeout_minutes: Math.round(n) }));
  }

  function timedOut(until: string | null) {
    return !!until && Date.parse(until) > Date.now();
  }

  let emojiName = $state('');
  let emojiKind = $state<'emoji' | 'sticker'>('emoji');

  async function addEmoji(e: SubmitEvent) {
    e.preventDefault();
    const form = e.currentTarget as HTMLFormElement;
    const file = (form.elements.namedItem('file') as HTMLInputElement).files?.[0];
    if (!file) return;
    const data = new FormData();
    data.append('name', emojiName);
    data.append('kind', emojiKind);
    data.append('file', file);
    const res = await fetch(`${base()}/api/guilds/${g.id}/emojis`, { method: 'POST', headers: { Authorization: `Bearer ${getToken()}` }, body: data }).catch(() => null);
    if (!res?.ok) return toast((await res?.json().catch(() => null))?.error ?? t('Upload fehlgeschlagen'));
    emojiName = '';
    form.reset();
  }

  async function deleteEmoji(id: string, label: string) {
    if ((await ask({ title: t('Emoji löschen'), text: t('„{name}“ wird gelöscht und in alten Nachrichten nicht mehr angezeigt.', { name: label }), confirm: t('Löschen'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/emojis/${id}`));
  }

  $effect(() => {
    if (tab === 'invites') loadInvites();
    if (tab === 'bans') loadBans();
    if (tab === 'audit') loadAudit();
  });

  const date = new Intl.DateTimeFormat(locale(), { dateStyle: 'medium', timeStyle: 'short' });
</script>

<SettingsPage title={t('{server}-Einstellungen', { server: w('server') }).replace(/^./, (c) => c.toUpperCase())} sections={tabs} bind:current={tab} onclose={() => (app.modal = null)}>
  <div class="layout">
    <div class="pane">
      {#if tab === 'overview'}
        <div class="stack">
          {#if can(P.MANAGE_GUILD, g)}
            <form class="stack" onsubmit={saveOverview}>
              <div class="row">
                <Avatar id={g.id} name={g.name} image={g.icon} size={64} square />
                <label class="btn">{t('Icon ändern')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={pickIcon} hidden /></label>
              </div>
              <label class="field"><span>{t('Name')}</span><input class="input" bind:value={name} maxlength="60" required /></label>
              <label class="field"><span>{t('Beschreibung')}</span><textarea class="input" rows="3" maxlength="300" bind:value={description}></textarea></label>
              <button class="btn primary">{t('Änderungen speichern')}</button>
            </form>
          {:else}
            <div class="row">
              <Avatar id={g.id} name={g.name} image={g.icon} size={64} square />
              <div><h3>{g.name}</h3><p class="muted">{g.description || t('Keine Beschreibung')}</p></div>
            </div>
          {/if}
          <hr />
          {#if owner}
            <p class="muted">{t('Löschen entfernt alle Kanäle und Nachrichten endgültig.')}</p>
            <button class="btn danger" onclick={destroy}>{t('{server} löschen', { server: w('server') })}</button>
          {:else}
            <button class="btn danger" onclick={leave}>{t('{server} verlassen', { server: w('server') })}</button>
          {/if}
        </div>
      {:else if tab === 'roles'}
        <div class="roles">
          <div class="rolelist">
            {#each g.roles as r (r.id)}
              <div class="roleitem" class:on={r.id === roleId}>
                <button class="row grow" onclick={() => pickRole(r)}>
                  <span class="swatch" style:background={r.color ?? 'var(--muted)'}></span>
                  <span class="ellipsis">{r.name}</span>
                </button>
                {#if !r.is_default && editable(r)}
                  <button class="icon" onclick={() => move(r, -1)} aria-label={t('{name} nach oben', { name: r.name })}><Icon name="up" size={14} /></button>
                  <button class="icon" onclick={() => move(r, 1)} aria-label={t('{name} nach unten', { name: r.name })}><Icon name="down" size={14} /></button>
                {/if}
              </div>
            {/each}
            <button class="btn small" onclick={addRole}><Icon name="plus" size={14} /> {t('Rolle anlegen')}</button>
            <p class="muted note">{t('Weiter oben stehende Rollen haben Vorrang und können niedrigere verwalten.')} <a href={docs('permissions')} target="_blank" rel="noopener noreferrer">{t('Alle Rechte erklärt')}</a></p>
          </div>
          {#if role}
            <form class="stack" onsubmit={saveRole}>
              <fieldset disabled={!editable(role)} class="stack">
                {#if !role.is_default}
                  <label class="field"><span>{t('Name')}</span><input class="input" bind:value={draft.name} maxlength="40" required /></label>
                  <div class="row wrap">
                    <label class="row"><input type="checkbox" bind:checked={draft.colored} /> {t('Eigene Farbe')}</label>
                    {#if draft.colored}<input type="color" bind:value={draft.color} aria-label={t('Rollenfarbe')} />{/if}
                    <label class="row"><input type="checkbox" bind:checked={draft.hoist} /> {t('In der Mitgliederliste getrennt anzeigen')}</label>
                  </div>
                {:else}
                  <p class="muted">{t('Diese Rolle hat jedes Mitglied. Ihre Berechtigungen gelten für alle.')}</p>
                {/if}
                <div class="perms">
                  {#each PERMISSIONS as p (p.bit)}
                    <label class="perm">
                      <input
                        type="checkbox"
                        checked={has(draft.permissions, p.bit)}
                        disabled={!has(g.permissions, p.bit)}
                        onchange={(e) => (draft.permissions = e.currentTarget.checked ? draft.permissions | p.bit : draft.permissions & ~p.bit)}
                      />
                      <span><strong>{t(p.name)}</strong>{#if p.hint}<br /><small class="muted">{t(p.hint)}</small>{/if}</span>
                    </label>
                  {/each}
                </div>
                <div class="row">
                  <button class="btn primary">{t('Rolle speichern')}</button>
                  {#if !role.is_default}<button type="button" class="btn danger" onclick={deleteRole}>{t('Löschen')}</button>{/if}
                </div>
              </fieldset>
              {#if !editable(role)}<p class="muted">{t('Diese Rolle steht nicht unter deiner höchsten Rolle, du kannst sie nicht ändern.')}</p>{/if}
            </form>
          {:else}
            <p class="muted">{t('Wähle links eine Rolle aus.')}</p>
          {/if}
        </div>
      {:else if tab === 'members'}
        <div class="stack members">
          {#each g.members as m (m.id)}
            {@const label = m.nickname || m.display_name}
            <div class="member">
              <div class="row">
                <Avatar id={m.id} name={m.display_name} image={m.avatar} size={36} />
                <div class="grow">
                  <strong class="ellipsis">{label}</strong>
                  {#if m.id === g.owner_id}<span class="crown" title={t('Besitzer')}><Icon name="crown" size={14} /></span>{/if}
                  <p class="muted ellipsis">@{m.username}{timedOut(m.timeout_until) ? t(', Timeout bis {date}', { date: date.format(new Date(m.timeout_until!)) }) : ''}</p>
                </div>
                {#if (m.id === app.me!.id && can(P.CHANGE_NICKNAME, g)) || canModerate(g, m.id, P.MANAGE_NICKNAMES)}
                  <button class="btn small" onclick={() => nickname(m.id, m.nickname)}>{t('Spitzname')}</button>
                {/if}
                {#if canModerate(g, m.id, P.MODERATE_MEMBERS)}
                  {#if timedOut(m.timeout_until)}
                    <button class="btn small" onclick={() => attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { timeout_minutes: 0 }))}>{t('Timeout aufheben')}</button>
                  {:else}
                    <button class="btn small" onclick={() => timeout(m.id, label)}>{t('Timeout')}</button>
                  {/if}
                {/if}
                {#if canModerate(g, m.id, P.KICK_MEMBERS)}<button class="btn small danger" onclick={() => kick(m.id, label)}>{t('Entfernen')}</button>{/if}
                {#if canModerate(g, m.id, P.BAN_MEMBERS)}<button class="btn small danger" onclick={() => ban(m.id, label)}>{t('Bannen')}</button>{/if}
                {#if owner && m.id !== app.me!.id}<button class="btn small" onclick={() => transfer(m.id, label)}>{t('Besitz übertragen')}</button>{/if}
              </div>
              {#if can(P.MANAGE_ROLES, g) && g.roles.some((r) => !r.is_default)}
                <div class="chips">
                  {#each g.roles.filter((r) => !r.is_default) as r (r.id)}
                    <button class="chip" class:on={m.roles.includes(r.id)} style:--c={r.color ?? 'var(--muted)'} disabled={!editable(r)} aria-pressed={m.roles.includes(r.id)} onclick={() => toggleRole(m.id, m.roles, r.id)}>{r.name}</button>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {:else if tab === 'invites'}
        <div class="stack">
          <div class="row wrap">
            <label class="field"><span>{t('Gültig')}</span>
              <select class="input" bind:value={hours}>
                <option value={1}>{t('1 Stunde')}</option>
                <option value={24}>{t('1 Tag')}</option>
                <option value={168}>{t('7 Tage')}</option>
                <option value={0}>{t('Unbegrenzt')}</option>
              </select>
            </label>
            <label class="field"><span>{t('Nutzungen')}</span>
              <select class="input" bind:value={maxUses}>
                <option value={0}>{t('Unbegrenzt')}</option>
                <option value={1}>1</option>
                <option value={10}>10</option>
                <option value={100}>100</option>
              </select>
            </label>
            <button class="btn primary end" onclick={createInvite}>{t('{invite} erstellen', { invite: w('invite') })}</button>
          </div>
          {#if created}
            <div class="row found">
              <code class="grow ellipsis">{link(created)}</code>
              <button class="btn small" onclick={() => copy(created!)}><Icon name="copy" size={14} /> {t('Kopieren')}</button>
            </div>
          {/if}
          {#each invites as inv (inv.code)}
            <div class="row line">
              <code>{inv.code}</code>
              <span class="muted grow ellipsis">{t('von {name}, {uses} genutzt', { name: inv.creator, uses: inv.uses + (inv.max_uses ? `/${inv.max_uses}` : '') })}{inv.expires_at ? t(', bis {date}', { date: date.format(new Date(inv.expires_at)) }) : ''}</span>
              <button class="icon" onclick={() => copy(inv.code)} aria-label={t('Link kopieren')}><Icon name="copy" size={15} /></button>
              <button class="icon" onclick={() => deleteInvite(inv.code)} aria-label={t('Einladung löschen')}><Icon name="trash" size={15} /></button>
            </div>
          {/each}
        </div>
      {:else if tab === 'emojis'}
        <div class="stack">
          <form class="row wrap" onsubmit={addEmoji}>
            <label class="field"><span>{t('Art')}</span>
              <select class="input" bind:value={emojiKind}>
                <option value="emoji">{t('Emoji')}</option>
                <option value="sticker">{t('Sticker')}</option>
              </select>
            </label>
            <label class="field grow"><span>{t('Name (a–z, 0–9, _)')}</span><input class="input" bind:value={emojiName} minlength="2" maxlength="32" pattern="[A-Za-z0-9_]+" required /></label>
            <label class="field"><span>{t('Bild')}</span><input name="file" type="file" accept="image/png,image/jpeg,image/gif,image/webp" required /></label>
            <button class="btn primary end">{t('Hochladen')}</button>
          </form>
          <p class="muted">{t('Emojis schreibst du im Chat als')} <code>{t(':name:')}</code>{t(', Sticker findest du im Emoji-Menü. Emojis bis 1 MB, Sticker bis 2 MB.')}</p>
          {#each ['emoji', 'sticker'] as kind}
            {@const list = g.emojis.filter((e) => e.kind === kind)}
            <h3>{kind === 'emoji' ? t('Emojis') : t('Sticker')} ({list.length})</h3>
            <div class="emojis">
              {#each list as e (e.id)}
                <div class="emoji" class:sticker={kind === 'sticker'}>
                  <img src={asset(e.file)} alt={e.name} />
                  <span class="ellipsis">{kind === 'emoji' ? `:${e.name}:` : e.name}</span>
                  <button class="icon" onclick={() => deleteEmoji(e.id, e.name)} aria-label={t('{name} löschen', { name: e.name })}><Icon name="trash" size={14} /></button>
                </div>
              {:else}
                <p class="muted">{t('Noch nichts hochgeladen.')}</p>
              {/each}
            </div>
          {/each}
        </div>
      {:else if tab === 'audit'}
        <div class="stack">
          {#each audit as a (a.id)}
            <div class="line">
              <strong>{a.actor}</strong> {ACTIONS[a.action] ? t(ACTIONS[a.action]) : a.action}{a.target ? ` ${a.target}` : ''}{a.detail ? ` (${auditDetail(a.detail)})` : ''}
              <p class="muted">{date.format(new Date(a.created_at))}</p>
            </div>
          {:else}
            <p class="muted">{t('Noch keine Einträge.')}</p>
          {/each}
        </div>
      {:else if tab === 'bans'}
        <div class="stack">
          {#each bans as b (b.id)}
            <div class="row line">
              <div class="grow">
                <strong>{b.display_name}</strong> <span class="muted">@{b.username}</span>
                <p class="muted">{b.reason || t('Kein Grund angegeben')}</p>
              </div>
              <button class="btn small" onclick={() => unban(b.id)}>{t('Bann aufheben')}</button>
            </div>
          {:else}
            <p class="muted">{t('Niemand ist gebannt.')}</p>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</SettingsPage>

<style>
  .layout {
    min-height: 100%;
  }
  .pane {
    min-width: 0;
  }
  hr {
    width: 100%;
    border: 0;
    border-top: 1px solid var(--line);
    margin: 0;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .roles {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 20px;
    align-items: start;
  }
  .rolelist {
    display: grid;
    gap: 4px;
  }
  .roleitem {
    display: flex;
    align-items: center;
    border-radius: var(--radius-s);
  }
  .roleitem > .row {
    padding: 6px 8px;
    min-width: 0;
    text-align: left;
  }
  .roleitem.on,
  .roleitem:hover {
    background: var(--panel-2);
  }
  .note {
    font-size: 0.85em;
    margin-top: 8px;
  }
  .swatch {
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
  }
  .icon {
    padding: 4px;
    border-radius: var(--radius-s);
    color: var(--muted);
  }
  .icon:hover {
    color: var(--ink);
  }
  input[type='color'] {
    width: 44px;
    height: 30px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--radius-s);
    background: none;
  }
  input[type='checkbox'] {
    width: 1.1em;
    height: 1.1em;
    accent-color: var(--accent);
  }
  .perms {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 10px 16px;
  }
  .perm {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    line-height: 1.3;
  }
  .perm input {
    margin-top: 3px;
    flex: none;
  }
  .member {
    padding-bottom: 12px;
    border-bottom: 1px solid var(--line);
  }
  .member > .row {
    flex-wrap: wrap;
  }
  .crown {
    color: var(--accent);
    display: inline-block;
    vertical-align: middle;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0 0 46px;
  }
  .chip {
    padding: 1px 10px;
    border-radius: 99px;
    border: 1px dashed var(--line);
    font-size: 0.85em;
    color: var(--muted);
  }
  .chip.on {
    border: 1px solid var(--c);
    color: var(--ink);
    font-weight: 600;
    background: color-mix(in srgb, var(--c) 18%, transparent);
  }
  .chip:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .emojis {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }
  .emoji {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
    min-width: 0;
  }
  .emoji img {
    width: 28px;
    height: 28px;
    object-fit: contain;
    flex: none;
  }
  .emoji.sticker img {
    width: 56px;
    height: 56px;
  }
  .emoji span {
    flex: 1;
  }
  .field.grow {
    min-width: 170px;
  }
  .end {
    align-self: flex-end;
  }
  .found {
    padding: 10px;
    border-radius: var(--radius-s);
    background: var(--panel-2);
  }
  .line {
    padding-bottom: 10px;
    border-bottom: 1px solid var(--line);
  }
  code {
    font-family: ui-monospace, monospace;
    font-size: 0.9em;
  }
  @media (max-width: 700px) {
    .roles {
      grid-template-columns: 1fr;
    }
    .chips {
      margin-left: 0;
    }
  }
</style>
