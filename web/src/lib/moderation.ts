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
  const nick = await askText({ title: t('Spitzname ändern'), input: { value: m.nickname ?? '', label: t('Spitzname'), placeholder: m.display_name }, text: t('Leer lassen, um den Spitznamen zu entfernen.'), confirm: t('Speichern') });
  if (nick !== null) await attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { nickname: nick }));
}

export async function timeout(g: Guild, m: Member) {
  const minutes = await askText({ title: t('Timeout für {name}', { name: label(m) }), text: t('Dauer in Minuten, z. B. 10, 60 oder 1440.'), input: { value: '60', type: 'number', label: t('Minuten') }, confirm: t('Timeout') });
  if (minutes === null) return;
  const n = Number(minutes);
  if (!Number.isFinite(n) || n <= 0) return toast(t('Bitte eine Zahl größer 0 eingeben'));
  await attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { timeout_minutes: Math.round(n) }));
}

export async function endTimeout(g: Guild, m: Member) {
  await attempt(api('PATCH', `/guilds/${g.id}/members/${m.id}`, { timeout_minutes: 0 }));
}

export async function kick(g: Guild, m: Member) {
  if ((await ask({ title: t('{name} entfernen', { name: label(m) }), text: t('{name} kann über eine neue Einladung wieder beitreten.', { name: label(m) }), confirm: t('Entfernen'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${m.id}`));
}

export async function ban(g: Guild, m: Member) {
  const reason = await askText({ title: t('{name} bannen', { name: label(m) }), text: t('{name} wird entfernt und kann nicht wieder beitreten, bis der Bann aufgehoben wird.', { name: label(m) }), input: { label: t('Grund (optional)') }, confirm: t('Bannen'), danger: true });
  if (reason !== null) await attempt(api('PUT', `/guilds/${g.id}/bans/${m.id}`, { reason }));
}

export async function transfer(g: Guild, m: Member) {
  if ((await ask({ title: t('Besitz übertragen'), text: t('{name} wird neuer Besitzer. Das lässt sich nur vom neuen Besitzer rückgängig machen.', { name: label(m) }), confirm: t('Übertragen'), danger: true }))) await attempt(api('PATCH', `/guilds/${g.id}`, { owner_id: m.id }));
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
  toast(t('In die Zwischenablage kopiert'));
}
