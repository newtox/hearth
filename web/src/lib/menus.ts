import { ask, askText } from './dialog.svelte';
import { api, inviteUrl } from './api';
import { t } from './i18n.svelte';
import { w } from './theme.svelte';
import { app, attempt, can, canModerate, isBlocked, isFriend, openDm, select, setStatus, toast, P, type Channel, type Dm, type Guild, type MenuItem, type Status } from './store.svelte';
import * as mod from './moderation';
import { canCall, joinVoice, setInputVolume, setUserVolume, voice } from './voice.svelte';

type Entry = MenuItem | false | null | undefined | 0;

const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

function tidy(items: Entry[]): MenuItem[] {
  const out: MenuItem[] = [];
  for (const it of items) {
    if (!it) continue;
    if (it === 'sep' && (!out.length || out[out.length - 1] === 'sep')) continue;
    out.push(it);
  }
  while (out[out.length - 1] === 'sep') out.pop();
  return out;
}

export function openMenu(e: MouseEvent, items: Entry[]) {
  const list = tidy(items);
  if (!list.length) return;
  e.preventDefault();
  e.stopPropagation();
  app.menu = { x: e.clientX, y: e.clientY, items: list };
}

export function openBelow(el: HTMLElement, items: Entry[]) {
  const list = tidy(items);
  if (!list.length) return;
  const r = el.getBoundingClientRect();
  app.menu = { x: r.left + 8, y: r.bottom + 4, items: list };
}

export function memberMenu(userId: string, g: Guild | null): Entry[] {
  const me = app.me!.id;
  const m = g?.members.find((x) => x.id === userId);
  const roles = g ? g.roles.filter((r) => !r.is_default) : [];
  const timed = m ? mod.timedOut(m) : false;
  return [
    { label: t('common_profile'), icon: 'user', action: () => (app.profile = userId) },
    g && m && { label: t('common_mention'), icon: 'at', action: () => (app.insert = `@${m.username} `) },
    userId !== me && { label: t('common_send_message'), icon: 'message', action: () => openDm(userId) },
    'sep',
    canCall && (userId === me ? { label: t('voice_input_volume'), range: { max: 2, value: () => voice.inputVolume, set: setInputVolume } } : { label: t('voice_user_volume'), range: { max: 2, value: () => voice.userVolume[userId] ?? 1, set: (v: number) => setUserVolume(userId, v) } }),
    'sep',
    g && m && ((userId === me && can(P.CHANGE_NICKNAME, g)) || canModerate(g, userId, P.MANAGE_NICKNAMES)) && { label: t('common_change_nickname'), icon: 'edit', action: () => mod.nickname(g, m) },
    g &&
      m &&
      can(P.MANAGE_ROLES, g) &&
      roles.length > 0 && {
        label: w('roles'),
        icon: 'tag',
        items: roles.map((r) => ({
          label: r.name,
          color: r.color,
          keep: true,
          checked: () => mod.hasRole(g, userId, r.id),
          disabled: () => !mod.roleEditable(g, r),
          action: () => mod.toggleRole(g, userId, r.id),
        })),
      },
    'sep',
    g && m && mod.isModerator(g) && { label: t('menu_open_moderator_view'), icon: 'eye', action: () => (app.modal = { kind: 'mod', userId }) },
    g && m && canModerate(g, userId, P.MODERATE_MEMBERS) && (timed ? { label: t('common_end_timeout'), icon: 'clock', action: () => mod.endTimeout(g, m) } : { label: t('common_time_out_name', { name: mod.label(m) }), icon: 'clock', danger: true, action: () => mod.timeout(g, m) }),
    g && m && canModerate(g, userId, P.KICK_MEMBERS) && { label: t('common_remove_name', { name: mod.label(m) }), icon: 'userMinus', danger: true, action: () => mod.kick(g, m) },
    g && m && canModerate(g, userId, P.BAN_MEMBERS) && { label: t('common_ban_name', { name: mod.label(m) }), icon: 'ban', danger: true, action: () => mod.ban(g, m) },
    g && m && g.owner_id === me && userId !== me && { label: t('common_transfer_ownership'), icon: 'crown', action: () => mod.transfer(g, m) },
    'sep',
    { label: t('common_copy_user_id'), icon: 'id', action: () => mod.copy(userId) },
  ];
}

function personOf(userId: string) {
  return app.friends.friends.find((u) => u.id === userId) ?? app.dms.flatMap((d) => d.recipients).find((u) => u.id === userId) ?? null;
}

async function call(userId: string, dm?: Dm) {
  const d = dm ?? (await openDm(userId));
  if (!d) return;
  if (dm) select(null, d.id);
  joinVoice(d);
}

async function editNote(userId: string) {
  const current = (await attempt(api<{ note?: string }>('GET', `/users/${userId}`)))?.note ?? '';
  const next = await askText({ title: t('menu_edit_note'), text: t('menu_note_only_you'), input: { value: current, label: t('menu_note'), placeholder: t('menu_note_placeholder') }, confirm: t('common_save') });
  if (next === null || next === undefined || next === current) return;
  if ((await attempt(api('PUT', `/users/${userId}/note`, { note: next }))) !== undefined) toast(t('menu_note_saved'));
}

async function inviteTo(g: Guild, userId: string) {
  const inv = await attempt(api<{ code: string }>('POST', `/guilds/${g.id}/invites`, { expires_in_hours: 168, max_uses: 1 }));
  if (!inv) return;
  const dm = await openDm(userId, false);
  if (!dm) return;
  if ((await attempt(api('POST', `/channels/${dm.id}/messages`, { content: inviteUrl(inv.code) }))) !== undefined) toast(t('menu_invite_sent', { server: g.name }));
}

async function unfriend(userId: string) {
  const name = personOf(userId)?.display_name ?? '';
  if (await ask({ title: t('common_remove_friend'), text: t('friends_pane_really_remove_name_as_friend', { name }), confirm: t('common_remove'), danger: true })) await attempt(api('DELETE', `/friends/${userId}`));
}

async function block(userId: string) {
  const name = personOf(userId)?.display_name ?? '';
  if (await ask({ title: t('profile_card_block_name', { name }), text: t('profile_card_no_longer_able_message_each'), confirm: t('profile_card_block'), danger: true })) await attempt(api('PUT', `/blocks/${userId}`));
}

async function addFriend(userId: string) {
  const u = personOf(userId);
  if (u && (await attempt(api('POST', '/friends', { username: u.username })))) toast(t('friends_pane_request_sent_name', { name: u.display_name }));
}

async function closeDm(dm: Dm) {
  await attempt(api('POST', `/channels/${dm.id}/close`));
}

export function friendMenu(userId: string, dm?: Dm): Entry[] {
  const guilds = app.guilds.filter((g) => can(P.CREATE_INVITE, g) && !g.members.some((m) => m.id === userId));
  const friend = isFriend(userId);
  const blocked = isBlocked(userId);
  const pending = app.friends.outgoing.some((u) => u.id === userId) || app.friends.incoming.some((u) => u.id === userId);
  return [
    dm && app.unread[dm.id] && { label: t('menu_mark_as_read'), icon: 'check', action: () => delete app.unread[dm.id] },
    'sep',
    { label: t('common_profile'), icon: 'user', action: () => (app.profile = userId) },
    !dm && { label: t('common_send_message'), icon: 'message', action: () => openDm(userId) },
    canCall && !blocked && { label: t('menu_start_call'), icon: 'phone', action: () => call(userId, dm) },
    { label: t('menu_edit_note'), hint: t('menu_note_only_you'), icon: 'edit', action: () => editNote(userId) },
    canCall && { label: t('voice_user_volume'), range: { max: 2, value: () => voice.userVolume[userId] ?? 1, set: (v: number) => setUserVolume(userId, v) } },
    'sep',
    guilds.length > 0 && { label: t('menu_invite_to_server', { server: w('server') }), icon: 'userPlus', items: guilds.map((g) => ({ label: g.name, action: () => inviteTo(g, userId) })) },
    friend && { label: t('common_remove_friend'), icon: 'userMinus', action: () => unfriend(userId) },
    !friend && !blocked && !pending && { label: t('menu_add_friend'), icon: 'userPlus', action: () => addFriend(userId) },
    dm && { label: t('menu_close_dm'), icon: 'close', action: () => closeDm(dm) },
    blocked ? { label: t('common_unblock'), icon: 'ban', action: () => attempt(api('DELETE', `/blocks/${userId}`)) } : { label: t('profile_card_block'), icon: 'ban', danger: true, action: () => block(userId) },
    'sep',
    { label: t('common_copy_user_id'), icon: 'id', action: () => mod.copy(userId) },
    dm && { label: t('menu_copy_channel_id'), icon: 'id', action: () => mod.copy(dm.id) },
  ];
}

export function groupMenu(dm: Dm): Entry[] {
  return [
    app.unread[dm.id] && { label: t('menu_mark_as_read'), icon: 'check', action: () => delete app.unread[dm.id] },
    { label: t('menu_group_settings'), icon: 'gear', action: () => (app.modal = { kind: 'groupchat', dm }) },
    'sep',
    { label: t('menu_copy_channel_id'), icon: 'id', action: () => mod.copy(dm.id) },
  ];
}

async function invite(g: Guild) {
  const inv = await attempt(api<{ code: string }>('POST', `/guilds/${g.id}/invites`, { expires_in_hours: 168, max_uses: 0 }));
  if (!inv) return;
  await mod.copy(inviteUrl(inv.code));
  toast(t('menu_invite_link_copied_valid_7'));
}

async function leave(g: Guild) {
  if ((await ask({ title: t('common_leave_name', { name: g.name }), text: t('common_only_rejoin_new_invite'), confirm: t('common_leave'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${app.me!.id}`));
}

export function guildMenu(g: Guild): Entry[] {
  const channels = can(P.MANAGE_CHANNELS, g);
  return [
    can(P.CREATE_INVITE, g) && { label: t('menu_invite_people'), icon: 'userPlus', action: () => invite(g) },
    { label: cap(t('common_server_settings', { server: w('server') })), icon: 'gear', action: () => (app.modal = { kind: 'guild' }) },
    'sep',
    channels && { label: t('menu_create_channel'), icon: 'hash', action: () => (app.modal = { kind: 'channel', channel: null, parent: null }) },
    channels && { label: t('menu_create_voice_channel'), icon: 'volume', action: () => (app.modal = { kind: 'channel', channel: null, type: 'voice' }) },
    channels && { label: t('menu_create_category'), icon: 'folder', action: () => (app.modal = { kind: 'channel', channel: null, type: 'category' }) },
    'sep',
    { label: t('menu_copy_server_id', { server: w('server') }), icon: 'id', action: () => mod.copy(g.id) },
    g.owner_id !== app.me?.id && 'sep',
    g.owner_id !== app.me?.id && { label: t('common_leave_server', { server: w('server') }), icon: 'leave', danger: true, action: () => leave(g) },
  ];
}

export function channelMenu(g: Guild, c: Channel): Entry[] {
  const manage = can(P.MANAGE_CHANNELS, g);
  const category = c.kind === 'category';
  return [
    !category && { label: t('menu_mark_as_read'), icon: 'check', action: () => delete app.unread[c.id] },
    'sep',
    manage && { label: category ? t('common_edit_category') : t('common_edit_name', { name: c.name }), icon: 'edit', action: () => (app.modal = { kind: 'channel', channel: c }) },
    manage && category && { label: t('menu_create_channel_category'), icon: 'plus', action: () => (app.modal = { kind: 'channel', channel: null, parent: c.id }) },
    'sep',
    { label: t('menu_copy_channel_id'), icon: 'id', action: () => mod.copy(c.id) },
  ];
}

const STATUSES: { id: Status; name: string; hint: string; color: string }[] = [
  { id: 'online', name: 'common_online', hint: '', color: 'var(--ok)' },
  { id: 'idle', name: 'common_idle', hint: '', color: '#f0b232' },
  { id: 'dnd', name: 'common_do_not_disturb', hint: 'menu_no_sounds_messages_calls', color: 'var(--danger)' },
  { id: 'invisible', name: 'common_invisible', hint: 'menu_appear_offline_but_still_use', color: 'var(--muted)' },
];

export function statusMenu(): Entry[] {
  return [
    ...STATUSES.map((s) => ({
      label: t(s.name),
      hint: s.hint ? t(s.hint) : undefined,
      color: s.color,
      checked: () => (app.me?.status ?? 'online') === s.id,
      action: () => setStatus(s.id),
    })),
    'sep',
    { label: t('common_profile'), icon: 'user', action: () => (app.profile = app.me!.id) },
    { label: t('common_settings'), icon: 'gear', action: () => (app.modal = { kind: 'user' }) },
  ];
}
