import { ask } from './dialog.svelte';
import { api } from './api';
import { t } from './i18n.svelte';
import { w } from './theme.svelte';
import { app, attempt, can, canModerate, openDm, setStatus, toast, P, type Channel, type Guild, type MenuItem, type Status } from './store.svelte';
import * as mod from './moderation';

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
    { label: t('Profil'), icon: 'user', action: () => (app.profile = userId) },
    g && m && { label: t('Erwähnen'), icon: 'at', action: () => (app.insert = `@${m.username} `) },
    userId !== me && { label: t('Nachricht senden'), icon: 'message', action: () => openDm(userId) },
    'sep',
    g && m && ((userId === me && can(P.CHANGE_NICKNAME, g)) || canModerate(g, userId, P.MANAGE_NICKNAMES)) && { label: t('Spitzname ändern'), icon: 'edit', action: () => mod.nickname(g, m) },
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
    g && m && mod.isModerator(g) && { label: t('In Moderatorenansicht öffnen'), icon: 'eye', action: () => (app.modal = { kind: 'mod', userId }) },
    g && m && canModerate(g, userId, P.MODERATE_MEMBERS) && (timed ? { label: t('Timeout aufheben'), icon: 'clock', action: () => mod.endTimeout(g, m) } : { label: t('Timeout für {name}', { name: mod.label(m) }), icon: 'clock', danger: true, action: () => mod.timeout(g, m) }),
    g && m && canModerate(g, userId, P.KICK_MEMBERS) && { label: t('{name} entfernen', { name: mod.label(m) }), icon: 'userMinus', danger: true, action: () => mod.kick(g, m) },
    g && m && canModerate(g, userId, P.BAN_MEMBERS) && { label: t('{name} bannen', { name: mod.label(m) }), icon: 'ban', danger: true, action: () => mod.ban(g, m) },
    g && m && g.owner_id === me && userId !== me && { label: t('Besitz übertragen'), icon: 'crown', action: () => mod.transfer(g, m) },
    'sep',
    { label: t('Nutzer-ID kopieren'), icon: 'id', action: () => mod.copy(userId) },
  ];
}

async function invite(g: Guild) {
  const inv = await attempt(api<{ code: string }>('POST', `/guilds/${g.id}/invites`, { expires_in_hours: 168, max_uses: 0 }));
  if (!inv) return;
  await mod.copy(`${location.origin}/#/invite/${inv.code}`);
  toast(t('Einladungslink kopiert, gültig für 7 Tage'));
}

async function leave(g: Guild) {
  if ((await ask({ title: t('„{name}“ verlassen', { name: g.name }), text: t('Du kannst nur mit einer neuen Einladung wieder beitreten.'), confirm: t('Verlassen'), danger: true }))) await attempt(api('DELETE', `/guilds/${g.id}/members/${app.me!.id}`));
}

export function guildMenu(g: Guild): Entry[] {
  const channels = can(P.MANAGE_CHANNELS, g);
  return [
    can(P.CREATE_INVITE, g) && { label: t('Leute einladen'), icon: 'userPlus', action: () => invite(g) },
    { label: cap(t('{server}-Einstellungen', { server: w('server') })), icon: 'gear', action: () => (app.modal = { kind: 'guild' }) },
    'sep',
    channels && { label: t('Kanal erstellen'), icon: 'hash', action: () => (app.modal = { kind: 'channel', channel: null, parent: null }) },
    channels && { label: t('Sprachkanal erstellen'), icon: 'volume', action: () => (app.modal = { kind: 'channel', channel: null, type: 'voice' }) },
    channels && { label: t('Kategorie erstellen'), icon: 'folder', action: () => (app.modal = { kind: 'channel', channel: null, type: 'category' }) },
    'sep',
    { label: t('{server}-ID kopieren', { server: w('server') }), icon: 'id', action: () => mod.copy(g.id) },
    g.owner_id !== app.me?.id && 'sep',
    g.owner_id !== app.me?.id && { label: t('{server} verlassen', { server: w('server') }), icon: 'leave', danger: true, action: () => leave(g) },
  ];
}

export function channelMenu(g: Guild, c: Channel): Entry[] {
  const manage = can(P.MANAGE_CHANNELS, g);
  const category = c.kind === 'category';
  return [
    !category && { label: t('Als gelesen markieren'), icon: 'check', action: () => delete app.unread[c.id] },
    'sep',
    manage && { label: category ? t('Kategorie bearbeiten') : t('{name} bearbeiten', { name: c.name }), icon: 'edit', action: () => (app.modal = { kind: 'channel', channel: c }) },
    manage && category && { label: t('Kanal in dieser Kategorie erstellen'), icon: 'plus', action: () => (app.modal = { kind: 'channel', channel: null, parent: c.id }) },
    'sep',
    { label: t('Kanal-ID kopieren'), icon: 'id', action: () => mod.copy(c.id) },
  ];
}

const STATUSES: { id: Status; name: string; hint: string; color: string }[] = [
  { id: 'online', name: 'Online', hint: '', color: 'var(--ok)' },
  { id: 'idle', name: 'Abwesend', hint: '', color: '#f0b232' },
  { id: 'dnd', name: 'Bitte nicht stören', hint: 'Keine Töne für Nachrichten und Anrufe', color: 'var(--danger)' },
  { id: 'invisible', name: 'Unsichtbar', hint: 'Du wirst als offline angezeigt, kannst aber alles nutzen', color: 'var(--muted)' },
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
    { label: t('Profil'), icon: 'user', action: () => (app.profile = app.me!.id) },
    { label: t('Einstellungen'), icon: 'gear', action: () => (app.modal = { kind: 'user' }) },
  ];
}
