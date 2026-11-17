import { t } from './i18n.svelte';
import { api, ApiError, gatewayUrl, getToken, setToken } from './api';
import { P, has } from './perms';
import { startDetection } from './activity';
import { current } from './current';
import { play } from './sounds';

export interface User {
  id: string;
  username: string;
  display_name: string;
  avatar: string | null;
  banner?: string | null;
  bio?: string;
  accent_color?: string | null;
  created_at?: string;
  online?: boolean;
  email?: string;
  is_admin?: boolean;
  voice?: boolean;
  badges?: Badge[];
  activity?: Activity | null;
  streaming?: boolean;
  spotify?: { available: boolean; linked: boolean };
  progress?: Progress;
}
export interface Progress {
  colors: string[];
  tracks: { track: string; icon: string; value: number; tier: number | null; steps: number[] }[];
}
export interface Activity {
  kind: 'playing' | 'listening' | 'watching' | 'custom';
  name: string;
  details: string;
  since: string;
}
export interface Badge {
  id: string;
  name: string;
  description: string;
  icon: string;
  color: string;
  track?: string;
  tier?: number;
  hours?: number;
}
export interface Emoji {
  id: string;
  kind: 'emoji' | 'sticker';
  name: string;
  file: string;
}
export interface Attachment {
  id: string;
  filename: string;
  file: string;
  size: number;
  is_image: boolean;
}
export interface Role {
  id: string;
  name: string;
  color: string | null;
  permissions: number;
  position: number;
  hoist: boolean;
  is_default: boolean;
}
export interface Overwrite {
  channel_id: string;
  target_id: string;
  kind: 'role' | 'member';
  allow: number;
  deny: number;
}
export interface Channel {
  id: string;
  guild_id: string | null;
  kind: 'text' | 'category' | 'voice';
  name: string;
  topic: string;
  parent_id: string | null;
  position: number;
  overwrites: Overwrite[];
  permissions: number;
}
export interface Member extends User {
  nickname: string | null;
  joined_at: string;
  timeout_until: string | null;
  roles: string[];
}
export interface Guild {
  id: string;
  name: string;
  description: string;
  icon: string | null;
  owner_id: string;
  roles: Role[];
  channels: Channel[];
  members: Member[];
  emojis: Emoji[];
  voice_states: { user_id: string; channel_id: string }[];
  permissions: number;
}
export interface Dm {
  id: string;
  kind: 'dm' | 'group';
  name: string;
  owner_id: string | null;
  icon: string | null;
  settings: Record<string, number>;
  roles: Record<string, number>;
  recipients: User[];
  voice: string[];
}
export interface Friends {
  friends: User[];
  incoming: User[];
  outgoing: User[];
  blocked: User[];
}
export interface Message {
  id: string;
  channel_id: string;
  content: string;
  created_at: string;
  edited_at: string | null;
  author: User;
  attachments: Attachment[];
  reactions: { emoji: string; users: string[] }[];
}
export type Modal =
  | { kind: 'create' ; code?: string }
  | { kind: 'guild'; tab?: string }
  | { kind: 'channel'; channel: Channel | null; parent?: string | null; type?: 'text' | 'category' | 'voice' }
  | { kind: 'user' }
  | { kind: 'groupchat'; dm: Dm | null }
  | null;

export const app = $state({
  authed: !!getToken(),
  ready: false,
  connected: false,
  me: null as User | null,
  guilds: [] as Guild[],
  dms: [] as Dm[],
  friends: { friends: [], incoming: [], outgoing: [], blocked: [] } as Friends,
  ring: null as { channelId: string; userId: string } | null,
  guildId: null as string | null,
  channelId: null as string | null,
  messages: {} as Record<string, Message[]>,
  more: {} as Record<string, boolean>,
  unread: {} as Record<string, boolean>,
  typing: {} as Record<string, Record<string, number>>,
  pane: 'chat' as 'side' | 'chat' | 'members',
  modal: null as Modal,
  profile: null as string | null,
  toast: '',
});

const lastChannel: Record<string, string> = {};
let ws: WebSocket | null = null;
let retry = 0;
let pingTimer: ReturnType<typeof setInterval> | undefined;
let toastTimer: ReturnType<typeof setTimeout> | undefined;
let ringTimer: ReturnType<typeof setTimeout> | undefined;

export function toast(message: string) {
  app.toast = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = ''), 4000);
}

export async function attempt<T>(p: Promise<T>): Promise<T | undefined> {
  try {
    return await p;
  } catch (e) {
    toast(e instanceof Error ? e.message : t('Unbekannter Fehler'));
    return undefined;
  }
}

export function guild() {
  return app.guilds.find((g) => g.id === app.guildId) ?? null;
}

export function channel(): Channel | Dm | null {
  const g = guild();
  if (g) return g.channels.find((c) => c.id === app.channelId) ?? null;
  return app.dms.find((d) => d.id === app.channelId) ?? null;
}

export async function refreshFriends() {
  const f = await attempt(api<Friends>('GET', '/friends'));
  if (f) app.friends = f;
}

export function isBlocked(id: string) {
  return app.friends.blocked.some((f) => f.id === id);
}

export function isFriend(id: string) {
  return app.friends.friends.some((f) => f.id === id);
}

export function dmTitle(dm: Dm) {
  if (dm.kind === 'dm') return partner(dm).display_name;
  return dm.name || dm.recipients.filter((r) => r.id !== app.me?.id).map((r) => r.display_name).join(', ') || t('Leere Gruppe');
}

export function partner(dm: Dm) {
  return dm.recipients.find((r) => r.id !== app.me?.id) ?? dm.recipients[0];
}

export function memberName(g: Guild | null, user: { id: string; display_name: string }) {
  return g?.members.find((m) => m.id === user.id)?.nickname || user.display_name;
}

export function memberColor(g: Guild | null, userId: string) {
  const m = g?.members.find((x) => x.id === userId);
  if (!g || !m) return null;
  return g.roles.find((r) => r.color && m.roles.includes(r.id))?.color ?? null;
}

export function topPosition(g: Guild, userId: string) {
  if (g.owner_id === userId) return Infinity;
  const m = g.members.find((x) => x.id === userId);
  return Math.max(0, ...g.roles.filter((r) => m?.roles.includes(r.id)).map((r) => r.position));
}

export function can(bit: number, g: Guild | null = guild()) {
  return has(g?.permissions, bit);
}

export function findEmoji(name: string, kind: 'emoji' | 'sticker' = 'emoji') {
  const lower = name.toLowerCase();
  const own = guild()?.emojis.find((e) => e.kind === kind && e.name === lower);
  if (own) return own;
  for (const g of app.guilds) {
    const e = g.emojis.find((x) => x.kind === kind && x.name === lower);
    if (e) return e;
  }
  return null;
}

export function guildUnread(g: Guild) {
  return g.channels.some((c) => app.unread[c.id]);
}

export function dmUnread() {
  return app.dms.some((d) => app.unread[d.id]);
}

export async function login(path: 'login' | 'register', body: Record<string, string>) {
  const res = await api('POST', `/auth/${path}`, body);
  setToken(res.token);
  app.authed = true;
  connect();
}

export async function logout() {
  await api('POST', '/auth/logout').catch(() => {});
  reset();
}

function reset() {
  setToken(null);
  ws?.close();
  ws = null;
  Object.assign(app, { authed: false, ready: false, connected: false, me: null, guilds: [], dms: [], guildId: null, channelId: null, messages: {}, more: {}, unread: {}, typing: {}, modal: null, profile: null });
}

export function connect() {
  if (!getToken() || (ws && ws.readyState <= WebSocket.OPEN)) return;
  const socket = new WebSocket(gatewayUrl());
  ws = socket;
  socket.onopen = () => socket.send(JSON.stringify({ op: 'identify', token: getToken() }));
  socket.onmessage = (ev) => {
    const { t, d } = JSON.parse(ev.data);
    handle(t, d);
  };
  socket.onclose = () => {
    if (ws !== socket) return;
    app.connected = false;
    clearInterval(pingTimer);
    if (getToken()) setTimeout(connect, Math.min(15000, 1000 * 2 ** retry++));
  };
  clearInterval(pingTimer);
  pingTimer = setInterval(() => socket.readyState === WebSocket.OPEN && socket.send('{"op":"ping"}'), 30000);
}

function patchUser(id: string, patch: Partial<User>) {
  if (app.me?.id === id) Object.assign(app.me, patch);
  for (const g of app.guilds) {
    const m = g.members.find((x) => x.id === id);
    if (m) Object.assign(m, patch);
  }
  for (const d of app.dms) {
    const r = d.recipients.find((x) => x.id === id);
    if (r) Object.assign(r, patch);
  }
  for (const list of [app.friends.friends, app.friends.incoming, app.friends.outgoing]) {
    const f = list.find((x) => x.id === id);
    if (f) Object.assign(f, patch);
  }
  for (const list of Object.values(app.messages)) {
    for (const m of list) if (m.author.id === id) Object.assign(m.author, patch);
  }
}

function spotifyNotice() {
  const result = new URLSearchParams(location.search).get('spotify');
  if (!result) return;
  toast(result === 'linked' ? t('Spotify ist verbunden') : t('Spotify konnte nicht verbunden werden'));
  history.replaceState(null, '', location.pathname);
}

function handle(t: string, d: any) {
  switch (t) {
    case 'READY': {
      retry = 0;
      app.me = d.user;
      app.guilds = d.guilds;
      app.dms = d.dms;
      app.friends = d.friends;
      app.messages = {};
      app.more = {};
      app.connected = true;
      app.ready = true;
      startDetection();
      spotifyNotice();
      const invite = location.hash.match(/^#\/invite\/(\w+)/);
      if (invite) {
        app.modal = { kind: 'create', code: invite[1] };
        history.replaceState(null, '', location.pathname);
      }
      if (channel() && channel()!.kind !== 'voice') loadMessages(app.channelId!);
      else select(app.guildId && guild() ? app.guildId : (app.guilds[0]?.id ?? null));
      break;
    }
    case 'INVALID_SESSION':
      reset();
      break;
    case 'MESSAGE_CREATE': {
      const list = app.messages[d.channel_id];
      if (list && !list.some((m) => m.id === d.id)) list.push(d);
      if (d.channel_id !== app.channelId || document.hidden) {
        if (d.author.id !== app.me?.id) {
          app.unread[d.channel_id] = true;
          if (app.dms.some((x) => x.id === d.channel_id) && !isBlocked(d.author.id)) play('message');
        }
      }
      if (app.typing[d.channel_id]) delete app.typing[d.channel_id][d.author.id];
      break;
    }
    case 'MESSAGE_UPDATE': {
      const list = app.messages[d.channel_id];
      const i = list?.findIndex((m) => m.id === d.id) ?? -1;
      if (i >= 0) list[i] = d;
      break;
    }
    case 'MESSAGE_DELETE': {
      const list = app.messages[d.channel_id];
      if (list) app.messages[d.channel_id] = list.filter((m) => m.id !== d.id);
      break;
    }
    case 'TYPING_START':
      if (d.user_id !== app.me?.id) (app.typing[d.channel_id] ??= {})[d.user_id] = Date.now() + 6000;
      break;
    case 'PRESENCE_UPDATE':
      patchUser(d.user_id, { online: d.online, activity: d.activity, streaming: d.streaming });
      break;
    case 'USER_UPDATE':
      patchUser(d.id, d);
      break;
    case 'DM_CREATE':
      if (!app.dms.some((x) => x.id === d.id)) app.dms.unshift(d);
      break;
    case 'DM_UPDATE': {
      const i = app.dms.findIndex((x) => x.id === d.id);
      if (i >= 0) app.dms[i] = d;
      break;
    }
    case 'DM_DELETE':
      app.dms = app.dms.filter((x) => x.id !== d.id);
      if (app.ring?.channelId === d.id) app.ring = null;
      if (!app.guildId && app.channelId === d.id) {
        app.modal = null;
        select(null);
      }
      break;
    case 'FRIENDS_UPDATE':
      refreshFriends();
      break;
    case 'CALL_RING':
      app.ring = { channelId: d.channel_id, userId: d.user_id };
      clearTimeout(ringTimer);
      ringTimer = setTimeout(() => (app.ring = null), 45000);
      break;
    case 'VOICE_STATE': {
      if (d.user_id !== app.me?.id && current.voiceChannel && d.room === current.voiceChannel) play(d.channel_id ? 'join' : 'leave');
      const dm = d.guild_id ? null : app.dms.find((x) => x.id === d.room);
      if (dm) {
        dm.voice = dm.voice.filter((u) => u !== d.user_id);
        if (d.channel_id) dm.voice.push(d.user_id);
        if (app.ring?.channelId === dm.id && (!dm.voice.length || d.user_id === app.me?.id)) app.ring = null;
      }
      const g = app.guilds.find((x) => x.id === d.guild_id);
      if (g) {
        g.voice_states = g.voice_states.filter((v) => v.user_id !== d.user_id);
        if (d.channel_id) g.voice_states.push({ user_id: d.user_id, channel_id: d.channel_id });
      }
      break;
    }
    case 'VOICE_WATCHERS':
      if (d.room === current.voiceChannel) current.onWatchers?.(d.watchers);
      break;
    case 'GUILD_REFRESH':
      refreshGuild(d.guild_id);
      break;
    case 'GUILD_DELETE':
      removeGuild(d.guild_id);
      break;
  }
}

function removeGuild(id: string) {
  app.guilds = app.guilds.filter((g) => g.id !== id);
  if (app.guildId === id) {
    app.modal = null;
    select(app.guilds[0]?.id ?? null);
  }
}

export function putGuild(g: Guild) {
  const i = app.guilds.findIndex((x) => x.id === g.id);
  if (i >= 0) app.guilds[i] = g;
  else app.guilds.push(g);
  if (app.guildId === g.id && !g.channels.some((c) => c.id === app.channelId && c.kind !== 'category')) select(g.id);
}

async function refreshGuild(id: string) {
  try {
    putGuild(await api('GET', `/guilds/${id}`));
  } catch (e) {
    if (e instanceof ApiError && e.status === 404) removeGuild(id);
  }
}

export function select(guildId: string | null, channelId?: string) {
  if (app.guildId && app.channelId) lastChannel[app.guildId] = app.channelId;
  app.guildId = guildId;
  const g = guild();
  let target = channelId;
  if (!target) {
    if (g) {
      const remembered = lastChannel[g.id];
      target = g.channels.find((c) => c.id === remembered && c.kind === 'text')?.id ?? g.channels.find((c) => c.kind === 'text')?.id;
    }
  }
  app.channelId = target ?? null;
  if (target) {
    delete app.unread[target];
    if (channelId) app.pane = 'chat';
    if (!app.messages[target] && (!g || g.channels.find((c) => c.id === target)?.kind === 'text')) loadMessages(target);
  }
}

export async function loadMessages(channelId: string, older = false) {
  const existing = app.messages[channelId] ?? [];
  const before = older && existing.length ? `&before=${existing[0].id}` : '';
  const page = await attempt(api<Message[]>('GET', `/channels/${channelId}/messages?limit=50${before}`));
  if (!page) return;
  app.more[channelId] = page.length === 50;
  app.messages[channelId] = older ? [...page, ...(app.messages[channelId] ?? [])] : page;
}

export async function openDm(userId: string) {
  const dm = await attempt(api<Dm>('POST', '/users/@me/channels', { user_id: userId }));
  if (!dm) return;
  if (!app.dms.some((d) => d.id === dm.id)) app.dms.unshift(dm);
  app.profile = null;
  app.modal = null;
  select(null, dm.id);
}

export function canModerate(g: Guild, targetId: string, bit: number) {
  if (!app.me || targetId === app.me.id || targetId === g.owner_id) return false;
  return has(g.permissions, bit) && topPosition(g, app.me.id) > topPosition(g, targetId);
}

export { P };

document.addEventListener('visibilitychange', () => {
  if (!document.hidden && app.channelId) delete app.unread[app.channelId];
});

setInterval(() => {
  const now = Date.now();
  for (const users of Object.values(app.typing)) {
    for (const [id, until] of Object.entries(users)) if (until < now) delete users[id];
  }
}, 2000);

if (getToken()) connect();
