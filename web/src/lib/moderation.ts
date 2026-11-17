import { ask, askText } from './dialog.svelte';
import { api } from './api';
import { t } from './i18n.svelte';
import { app, attempt, topPosition, toast, type Guild, type Member, type Role } from './store.svelte';
import { P, has } from './perms';

export function label(m: Member) {
  return m.nickname || m.display_name;
}

export function timedOut(m: Member) {
  return !!m.timeout_until && Date.parse(m.timeout_until) > Date.now();
}

export function roleEditable(g: Guild, r: Role) {
  return g.owner_id === app.me?.id || r.position < topPosition(g, app.me!.id);
}

export function isModerator(g: Guild) {
  return [P.ADMINISTRATOR, P.MODERATE_MEMBERS, P.KICK_MEMBERS, P.BAN_MEMBERS, P.MANAGE_ROLES, P.MANAGE_NICKNAMES, P.MANAGE_MESSAGES].some((bit) => has(g.permissions, bit));
}

export function memberPermissions(g: Guild, m: Member) {
  if (m.id === g.owner_id) return -1;
  const bits = g.roles.filter((r) => r.is_default || m.roles.includes(r.id)).reduce((acc, r) => acc | r.permissions, 0);
  return bits & P.ADMINISTRATOR ? -1 : bits;
}

export async function setRoles(g: Guild, memberId: string, roles: string[]) {
  return attempt(api('PATCH', `/guilds/${g.id}/members/${memberId}`, { roles }));
}

export function fresh(g: Guild) {
  return app.guilds.find((x) => x.id === g.id) ?? g;
}

export function hasRole(g: Guild, memberId: string, roleId: string) {
  return !!fresh(g).members.find((x) => x.id === memberId)?.roles.includes(roleId);
}

export async function toggleRole(g: Guild, memberId: string, roleId: string) {
  g = fresh(g);
  const m = g.members.find((x) => x.id === memberId);
  if (!m) return;
  await setRoles(g, memberId, m.roles.includes(roleId) ? m.roles.filter((x) => x !== roleId) : [...m.roles, roleId]);
}

export async function nickname(g: Guild, m: Member) {
  const nick = await askText({ title: t('common_change_nickname'), input: { value: m.nickname ?? '', label: t('common_nickname'), placeholder: m.display_name }, text: t('common_leave_empty_remove_nickname'), confirm: t('common_save') });
  if (nick !== null) await attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { nickname: nick }));
}

export async function timeout(g: Guild, m: Member) {
  const minutes = await askText({ title: t('common_time_out_name', { name: label(m) }), text: t('common_duration_minutes_e_g_10'), input: { value: '60', type: 'number', label: t('common_minutes') }, confirm: t('common_timeout') });
  if (minutes === null) return;
  const n = Number(minutes);
  if (!Number.isFinite(n) || n <= 0) return toast(t('common_please_enter_number_greater_than'));
  await attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { timeout_minutes: Math.round(n) }));
}

export async function endTimeout(g: Guild, m: Member) {
  await attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { timeout_minutes: 0 }));
}

export async function kick(g: Guild, m: Member) {
  if ((await ask({ title: t('common_remove_name', { name: label(m) }), text: t('common_name_rejoin_new_invite', { name: label(m) }), confirm: t('common_remove'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${m.id}`));
}

export async function ban(g: Guild, m: Member) {
  const reason = await askText({ title: t('common_ban_name', { name: label(m) }), text: t('common_name_removed_cannot_rejoin_until', { name: label(m) }), input: { label: t('common_reason_optional') }, confirm: t('common_ban'), danger: true });
  if (reason !== null) await attempt(api('PUT', `/guilds/${g.id}/bans/${m.id}`, { reason }));
}

export async function transfer(g: Guild, m: Member) {
  if ((await ask({ title: t('common_transfer_ownership'), text: t('common_name_becomes_new_owner_only', { name: label(m) }), confirm: t('common_transfer'), danger: true }))) await attempt(api('PATCH', `/guilds/${g.id}`, { owner_id: m.id }));
}

export async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const area = document.createElement('textarea');
    area.value = text;
    document.body.append(area);
    area.select();
    document.execCommand('copy');
    area.remove();
  }
  toast(t('mod_copied_clipboard'));
}
