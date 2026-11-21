<script lang="ts">
  import { ask, askText } from '../lib/dialog.svelte';
  import { auditDetail, locale, serverError, t } from '../lib/i18n.svelte';
  import { untrack } from 'svelte';
  import { api, asset, base, getToken, inviteUrl, upload } from '../lib/api';
  import { PERMISSIONS, has } from '../lib/perms';
  import { app, attempt, can, canModerate, guild, toast, topPosition, P, type Role } from '../lib/store.svelte';
  import { w } from '../lib/theme.svelte';
  import { docs } from '../lib/docs';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import { sortable, type Place } from '../lib/drag';
  import { pickImage } from '../lib/crop.svelte';
  import SettingsPage from './SettingsPage.svelte';
  import UnsavedBar from './UnsavedBar.svelte';
  import { leave as leaveUnsaved } from '../lib/unsaved.svelte';
  import { onDestroy } from 'svelte';

  let { tab: initial = 'overview' }: { tab?: string } = $props();

  const g = $derived(guild()!);
  const owner = $derived(g.owner_id === app.me!.id);
  const myTop = $derived(topPosition(g, app.me!.id));
  const tabs = $derived(
    [
      { id: 'overview', icon: 'gear', name: t('guild_settings_overview'), group: g.name, show: true },
      { id: 'roles', icon: 'shield', name: w('roles'), group: g.name, show: can(P.MANAGE_ROLES, g) },
      { id: 'emojis', icon: 'smile', name: t('common_emojis'), group: g.name, show: can(P.MANAGE_EMOJIS, g) },
      { id: 'members', icon: 'users', name: w('members'), group: t('guild_settings_member_management'), show: true },
      { id: 'invites', icon: 'link', name: w('invites'), group: t('guild_settings_member_management'), show: can(P.CREATE_INVITE, g) },
      { id: 'bans', icon: 'ban', name: t('guild_settings_bans'), group: t('guild_settings_member_management'), show: can(P.BAN_MEMBERS, g) },
      { id: 'audit', icon: 'list', name: t('guild_settings_audit_log'), group: t('guild_settings_moderation'), show: can(P.MANAGE_GUILD, g) },
      { id: 'leave', icon: 'leave', name: owner ? t('guild_settings_delete_server_named', { server: w('server') }) : t('common_leave_server', { server: w('server') }), danger: true, show: true, action: () => (owner ? destroy() : leave()) },
    ].filter((x) => x.show),
  );
  let tab = $state(untrack(() => initial));

  let name = $state(guild()!.name);
  let description = $state(guild()!.description);

  let iconDraft = $state<{ file: File; url: string } | 'remove' | null>(null);
  let savingOverview = $state(false);
  const shownIcon = $derived(iconDraft === 'remove' ? null : (iconDraft?.url ?? g.icon));
  const overviewDirty = $derived(name !== g.name || description !== g.description || iconDraft !== null);

  function setIcon(next: typeof iconDraft) {
    if (iconDraft && iconDraft !== 'remove') URL.revokeObjectURL(iconDraft.url);
    iconDraft = next;
  }

  function resetOverview() {
    name = g.name;
    description = g.description;
    setIcon(null);
  }

  async function saveOverviewNow() {
    if (savingOverview) return;
    savingOverview = true;
    try {
      if (iconDraft) {
        const ok = iconDraft === 'remove' ? await attempt(api('DELETE', `/guilds/${g.id}/icon`)) : await attempt(upload(`/guilds/${g.id}/icon`, iconDraft.file));
        if (ok === undefined) return;
        setIcon(null);
      }
      if (await attempt(api('PATCH', `/guilds/${g.id}`, { name, description }))) toast(t('guild_settings_saved'));
    } finally {
      savingOverview = false;
    }
  }

  function saveOverview(e: SubmitEvent) {
    e.preventDefault();
    saveOverviewNow();
  }

  async function pickIcon(e: Event) {
    const file = await pickImage(e, 'square');
    if (file) setIcon({ file, url: URL.createObjectURL(file) });
  }

  function clearIcon() {
    setIcon(g.icon ? 'remove' : null);
  }

  onDestroy(() => setIcon(null));

  async function leave() {
    if ((await ask({ title: t('common_leave_name', { name: g.name }), text: t('common_only_rejoin_new_invite'), confirm: t('common_leave'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${app.me!.id}`));
  }

  async function destroy() {
    if ((await askText({ title: t('guild_settings_delete_name', { name: g.name }), text: t('guild_settings_all_channels_messages_roles_lost'), input: { label: t('guild_settings_enter_server_name_confirm'), match: g.name }, confirm: t('guild_settings_delete_server'), danger: true })) === g.name) await attempt(api('DELETE', `/guilds/${g.id}`));
  }

  let roleId = $state<string | null>(null);
  const role = $derived(g.roles.find((r) => r.id === roleId) ?? null);
  let draft = $state({ name: '', color: '#8888aa', colored: false, hoist: false, permissions: 0 });

  function editable(r: Role) {
    return owner || r.position < myTop;
  }

  const roleDirty = $derived(!!role && (draft.name !== role.name || draft.colored !== !!role.color || (draft.colored && draft.color !== role.color) || draft.hoist !== role.hoist || draft.permissions !== role.permissions));

  function resetRole() {
    if (role) pickRole(role);
  }

  async function chooseRole(r: Role) {
    if (r.id === roleId || (await leaveUnsaved())) pickRole(r);
  }

  function pickRole(r: Role) {
    roleId = r.id;
    draft = { name: r.name, color: r.color ?? '#8888aa', colored: !!r.color, hoist: r.hoist, permissions: r.permissions };
  }

  async function addRole() {
    if (!(await leaveUnsaved())) return;
    const r = await attempt(api<Role>('POST', `/guilds/${g.id}/roles`, { name: t('guild_settings_new_role') }));
    if (r) pickRole(r);
  }

  function saveRole(e: SubmitEvent) {
    e.preventDefault();
    saveRoleNow();
  }

  async function saveRoleNow() {
    if (!role) return;
    const body = { name: draft.name, color: draft.colored ? draft.color : '', hoist: draft.hoist, permissions: draft.permissions };
    if (await attempt(api('PATCH', `/guilds/${g.id}/roles/${role.id}`, body))) toast(t('guild_settings_role_saved'));
  }

  async function deleteRole() {
    if (role && (await ask({ title: t('guild_settings_delete_role'), text: t('guild_settings_name_deleted_removed_from_all', { name: role.name }), confirm: t('common_delete'), danger: true }))) {
      await attempt(api('DELETE', `/guilds/${g.id}/roles/${role.id}`));
      roleId = null;
    }
  }

  async function saveOrder(order: string[]) {
    const before = g.roles;
    const rest = g.roles.filter((x) => x.is_default);
    g.roles = [...order.map((id, i) => ({ ...g.roles.find((x) => x.id === id)!, position: order.length - i })), ...rest];
    if (!(await attempt(api('PUT', `/guilds/${g.id}/roles`, order)))) g.roles = before;
  }

  function move(r: Role, dir: -1 | 1) {
    const order = g.roles.filter((x) => !x.is_default).map((x) => x.id);
    const i = order.indexOf(r.id);
    const j = i + dir;
    const other = g.roles.find((x) => x.id === order[j]);
    if (j < 0 || j >= order.length || !other || !editable(other)) return;
    [order[i], order[j]] = [order[j], order[i]];
    saveOrder(order);
  }

  function rolePlace(dragId: string, targetId: string, ratio: number): Place | null {
    const target = g.roles.find((x) => x.id === targetId);
    if (dragId === targetId || !target || target.is_default || !editable(target)) return null;
    return ratio < 0.5 ? 'before' : 'after';
  }

  function roleDrop(dragId: string, targetId: string, place: Place) {
    const order = g.roles.filter((x) => !x.is_default && x.id !== dragId).map((x) => x.id);
    const at = order.indexOf(targetId);
    order.splice(place === 'before' ? at : at + 1, 0, dragId);
    saveOrder(order);
  }

  function roleKey(e: KeyboardEvent, r: Role) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') || r.is_default || !editable(r)) return;
    e.preventDefault();
    move(r, e.key === 'ArrowUp' ? -1 : 1);
  }

  async function toggleRole(memberId: string, current: string[], id: string) {
    const roles = current.includes(id) ? current.filter((x) => x !== id) : [...current, id];
    await attempt(api('PATCH', `/guilds/${g.id}/members/${memberId}`, { roles }));
  }

  async function nickname(memberId: string, current: string | null) {
    const nick = await askText({ title: t('common_change_nickname'), input: { value: current ?? '', label: t('common_nickname'), placeholder: '' }, text: t('common_leave_empty_remove_nickname'), confirm: t('common_save') });
    if (nick !== null) await attempt(api('PATCH', `/guilds/${g.id}/members/${memberId}`, { nickname: nick }));
  }

  async function kick(id: string, label: string) {
    if ((await ask({ title: t('common_remove_name', { name: label }), text: t('common_name_rejoin_new_invite', { name: label }), confirm: t('common_remove'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${id}`));
  }

  async function ban(id: string, label: string) {
    const reason = await askText({ title: t('common_ban_name', { name: label }), text: t('common_name_removed_cannot_rejoin_until', { name: label }), input: { label: t('common_reason_optional') }, confirm: t('common_ban'), danger: true });
    if (reason !== null) await attempt(api('PUT', `/guilds/${g.id}/bans/${id}`, { reason }));
  }

  async function transfer(id: string, label: string) {
    if ((await ask({ title: t('common_transfer_ownership'), text: t('common_name_becomes_new_owner_only', { name: label }), confirm: t('common_transfer'), danger: true }))) await attempt(api('PATCH', `/guilds/${g.id}`, { owner_id: id }));
  }

  let invites = $state<any[]>([]);
  let created = $state<string | null>(null);
  let hours = $state(168);
  let maxUses = $state(0);
  let bans = $state<any[]>([]);

  function link(code: string) {
    return inviteUrl(code);
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
      toast(t('guild_settings_link_copied'));
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
    server_update: 'guild_settings_changed_server_settings',
    role_create: 'guild_settings_created_role',
    role_update: 'guild_settings_changed_role',
    role_delete: 'guild_settings_deleted_role',
    channel_create: 'guild_settings_created_channel',
    channel_update: 'guild_settings_changed_channel',
    channel_reorder: 'guild_settings_reordered_channels',
    channel_delete: 'guild_settings_deleted_channel',
    channel_rules: 'guild_settings_changed_channel_rules',
    member_roles: 'guild_settings_changed_roles',
    timeout: 'guild_settings_timed_out',
    timeout_end: 'guild_settings_ended_timeout',
    kick: 'guild_settings_kicked',
    ban: 'guild_settings_banned',
    unban: 'guild_settings_unbanned',
    message_delete: 'guild_settings_deleted_message',
    emoji_create: 'guild_settings_uploaded_emoji',
    sticker_create: 'guild_settings_uploaded_sticker',
    emoji_delete: 'guild_settings_deleted_emoji_sticker',
  };

  async function loadAudit() {
    audit = (await attempt(api('GET', `/guilds/${g.id}/audit`))) ?? [];
  }

  async function timeout(id: string, label: string) {
    const minutes = await askText({ title: t('common_time_out_name', { name: label }), text: t('common_duration_minutes_e_g_10'), input: { value: '60', type: 'number', label: t('common_minutes') }, confirm: t('common_timeout') });
    if (minutes === null) return;
    const n = Number(minutes);
    if (!Number.isFinite(n) || n <= 0) return toast(t('common_please_enter_number_greater_than'));
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
    if (!res?.ok) return toast((await res?.json().catch(() => null))?.error ?? t('guild_settings_upload_failed'));
    emojiName = '';
    form.reset();
  }

  async function deleteEmoji(id: string, label: string) {
    if ((await ask({ title: t('guild_settings_delete_emoji'), text: t('guild_settings_name_deleted_no_longer_shown', { name: label }), confirm: t('common_delete'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/emojis/${id}`));
  }

  $effect(() => {
    if (tab === 'invites') loadInvites();
    if (tab === 'bans') loadBans();
    if (tab === 'audit') loadAudit();
  });

  const date = new Intl.DateTimeFormat(locale(), { dateStyle: 'medium', timeStyle: 'short' });
</script>

<SettingsPage title={t('common_server_settings', { server: w('server') }).replace(/^./, (c) => c.toUpperCase())} sections={tabs} bind:current={tab} list={initial === 'overview'} onclose={() => (app.modal = null)}>
  <div class="layout">
    <div class="pane">
      {#if tab === 'overview'}
        <div class="stack">
          {#if can(P.MANAGE_GUILD, g)}
            <form class="stack" onsubmit={saveOverview}>
              <div class="row">
                <Avatar id={g.id} name={name || g.name} image={shownIcon} size={64} square />
                <label class="btn">{t('guild_settings_change_icon')}<input type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={pickIcon} hidden /></label>
                {#if shownIcon}<button type="button" class="btn" onclick={clearIcon}>{t('guild_settings_remove_icon')}</button>{/if}
              </div>
              <label class="field"><span>{t('common_name')}</span><input class="input" bind:value={name} maxlength="60" required /></label>
              <label class="field"><span>{t('common_description')}</span><textarea class="input" rows="3" maxlength="300" bind:value={description}></textarea></label>
              <button class="btn primary" disabled={!overviewDirty || savingOverview}>{t('common_save_changes')}</button>
              <UnsavedBar dirty={overviewDirty} busy={savingOverview} onreset={resetOverview} onsave={saveOverviewNow} />
            </form>
          {:else}
            <div class="row">
              <Avatar id={g.id} name={g.name} image={g.icon} size={64} square />
              <div><h3>{g.name}</h3><p class="muted">{g.description || t('guild_settings_no_description')}</p></div>
            </div>
          {/if}
          <hr />
          {#if owner}
            <p class="muted">{t('guild_settings_deleting_permanently_removes_all_channels')}</p>
            <button class="btn danger" onclick={destroy}>{t('guild_settings_delete_server_named', { server: w('server') })}</button>
          {:else}
            <button class="btn danger" onclick={leave}>{t('common_leave_server', { server: w('server') })}</button>
          {/if}
        </div>
      {:else if tab === 'roles'}
        <div class="roles">
          <div class="rolelist">
            {#each g.roles as r (r.id)}
              <div class="roleitem" class:on={r.id === roleId} class:movable={!r.is_default && editable(r)} use:sortable={{ id: r.id, group: 'roles', label: r.name, enabled: !r.is_default && editable(r), place: rolePlace, drop: roleDrop }}>
                <button class="row grow" onclick={() => chooseRole(r)} onkeydown={(e) => roleKey(e, r)} title={!r.is_default && editable(r) ? t('guild_settings_drag_to_sort') : undefined}>
                  <span class="swatch" style:background={r.color ?? 'var(--muted)'}></span>
                  <span class="ellipsis">{r.name}</span>
                </button>
                {#if !r.is_default && editable(r)}<span class="grip" aria-hidden="true"><Icon name="grip" size={14} /></span>{/if}
              </div>
            {/each}
            <button class="btn small" onclick={addRole}><Icon name="plus" size={14} /> {t('guild_settings_create_role')}</button>
            <p class="muted note">{t('guild_settings_roles_higher_up_take_precedence')} <a href={docs('permissions')} target="_blank" rel="noopener noreferrer">{t('guild_settings_all_permissions_explained')}</a></p>
          </div>
          {#if role}
            <form class="stack" onsubmit={saveRole}>
              <fieldset disabled={!editable(role)} class="stack">
                {#if !role.is_default}
                  <label class="field"><span>{t('common_name')}</span><input class="input" bind:value={draft.name} maxlength="40" required /></label>
                  <div class="row wrap">
                    <label class="row"><input type="checkbox" bind:checked={draft.colored} /> {t('guild_settings_custom_colour')}</label>
                    {#if draft.colored}<input type="color" bind:value={draft.color} aria-label={t('guild_settings_role_colour')} />{/if}
                    <label class="row"><input type="checkbox" bind:checked={draft.hoist} /> {t('guild_settings_show_separately_member_list')}</label>
                  </div>
                {:else}
                  <p class="muted">{t('guild_settings_every_member_has_role_its')}</p>
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
                <UnsavedBar dirty={roleDirty} onreset={resetRole} onsave={saveRoleNow} />
                <div class="row">
                  <button class="btn primary" disabled={!roleDirty}>{t('guild_settings_save_role')}</button>
                  {#if !role.is_default}<button type="button" class="btn danger" onclick={deleteRole}>{t('common_delete')}</button>{/if}
                </div>
              </fieldset>
              {#if !editable(role)}<p class="muted">{t('guild_settings_role_not_below_highest_role')}</p>{/if}
            </form>
          {:else}
            <p class="muted">{t('guild_settings_pick_role_left')}</p>
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
                  {#if m.id === g.owner_id}<span class="crown" title={t('common_owner')}><Icon name="crown" size={14} /></span>{/if}
                  <p class="muted ellipsis">@{m.username}{timedOut(m.timeout_until) ? t('guild_settings_timed_out_until_date', { date: date.format(new Date(m.timeout_until!)) }) : ''}</p>
                </div>
                {#if (m.id === app.me!.id && can(P.CHANGE_NICKNAME, g)) || canModerate(g, m.id, P.MANAGE_NICKNAMES)}
                  <button class="btn small" onclick={() => nickname(m.id, m.nickname)}>{t('common_nickname')}</button>
                {/if}
                {#if canModerate(g, m.id, P.MODERATE_MEMBERS)}
                  {#if timedOut(m.timeout_until)}
                    <button class="btn small" onclick={() => attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { timeout_minutes: 0 }))}>{t('common_end_timeout')}</button>
                  {:else}
                    <button class="btn small" onclick={() => timeout(m.id, label)}>{t('common_timeout')}</button>
                  {/if}
                {/if}
                {#if canModerate(g, m.id, P.KICK_MEMBERS)}<button class="btn small danger" onclick={() => kick(m.id, label)}>{t('common_remove')}</button>{/if}
                {#if canModerate(g, m.id, P.BAN_MEMBERS)}<button class="btn small danger" onclick={() => ban(m.id, label)}>{t('common_ban')}</button>{/if}
                {#if owner && m.id !== app.me!.id}<button class="btn small" onclick={() => transfer(m.id, label)}>{t('common_transfer_ownership')}</button>{/if}
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
            <label class="field"><span>{t('guild_settings_valid')}</span>
              <select class="input" bind:value={hours}>
                <option value={1}>{t('common_1_hour')}</option>
                <option value={24}>{t('common_1_day')}</option>
                <option value={168}>{t('common_7_days')}</option>
                <option value={0}>{t('common_unlimited')}</option>
              </select>
            </label>
            <label class="field"><span>{t('guild_settings_uses')}</span>
              <select class="input" bind:value={maxUses}>
                <option value={0}>{t('common_unlimited')}</option>
                <option value={1}>1</option>
                <option value={10}>10</option>
                <option value={100}>100</option>
              </select>
            </label>
            <button class="btn primary end" onclick={createInvite}>{t('guild_settings_create_invite', { invite: w('invite') })}</button>
          </div>
          {#if created}
            <div class="row found">
              <code class="grow ellipsis">{link(created)}</code>
              <button class="btn small" onclick={() => copy(created!)}><Icon name="copy" size={14} /> {t('guild_settings_copy')}</button>
            </div>
          {/if}
          {#each invites as inv (inv.code)}
            <div class="row line">
              <code>{inv.code}</code>
              <span class="muted grow ellipsis">{t('guild_settings_name_uses_used', { name: inv.creator, uses: inv.uses + (inv.max_uses ? `/${inv.max_uses}` : '') })}{inv.expires_at ? t('guild_settings_until_date', { date: date.format(new Date(inv.expires_at)) }) : ''}</span>
              <button class="icon" onclick={() => copy(inv.code)} aria-label={t('guild_settings_copy_link')}><Icon name="copy" size={15} /></button>
              <button class="icon" onclick={() => deleteInvite(inv.code)} aria-label={t('guild_settings_delete_invite')}><Icon name="trash" size={15} /></button>
            </div>
          {/each}
        </div>
      {:else if tab === 'emojis'}
        <div class="stack">
          <form class="row wrap" onsubmit={addEmoji}>
            <label class="field"><span>{t('common_type')}</span>
              <select class="input" bind:value={emojiKind}>
                <option value="emoji">{t('guild_settings_emoji')}</option>
                <option value="sticker">{t('common_stickers')}</option>
              </select>
            </label>
            <label class="field grow"><span>{t('guild_settings_name_z_0_9')}</span><input class="input" bind:value={emojiName} minlength="2" maxlength="32" pattern="[A-Za-z0-9_]+" required /></label>
            <label class="field"><span>{t('guild_settings_image')}</span><input name="file" type="file" accept="image/png,image/jpeg,image/gif,image/webp" required /></label>
            <button class="btn primary end">{t('guild_settings_upload')}</button>
          </form>
          <p class="muted">{t('guild_settings_type_emojis_chat_as')} <code>{t('guild_settings_name')}</code>{t('guild_settings_stickers_emoji_menu_emojis_up')}</p>
          {#each ['emoji', 'sticker'] as kind}
            {@const list = g.emojis.filter((e) => e.kind === kind)}
            <h3>{kind === 'emoji' ? t('common_emojis') : t('common_stickers')} ({list.length})</h3>
            <div class="emojis">
              {#each list as e (e.id)}
                <div class="emoji" class:sticker={kind === 'sticker'}>
                  <img src={asset(e.file)} alt={e.name} />
                  <span class="ellipsis">{kind === 'emoji' ? `:${e.name}:` : e.name}</span>
                  <button class="icon" onclick={() => deleteEmoji(e.id, e.name)} aria-label={t('common_delete_name', { name: e.name })}><Icon name="trash" size={14} /></button>
                </div>
              {:else}
                <p class="muted">{t('guild_settings_nothing_uploaded_yet')}</p>
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
            <p class="muted">{t('guild_settings_no_entries_yet')}</p>
          {/each}
        </div>
      {:else if tab === 'bans'}
        <div class="stack">
          {#each bans as b (b.id)}
            <div class="row line">
              <div class="grow">
                <strong>{b.display_name}</strong> <span class="muted">@{b.username}</span>
                <p class="muted">{b.reason || t('guild_settings_no_reason_given')}</p>
              </div>
              <button class="btn small" onclick={() => unban(b.id)}>{t('guild_settings_unban')}</button>
            </div>
          {:else}
            <p class="muted">{t('guild_settings_nobody_banned')}</p>
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
  .grip {
    display: inline-flex;
    padding: 0 8px;
    color: var(--muted);
    opacity: 0;
  }
  .roleitem:hover .grip {
    opacity: 1;
  }
  .roleitem.movable {
    cursor: grab;
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
