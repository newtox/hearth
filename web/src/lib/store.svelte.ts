import { t } from './i18n.svelte';
import { toParams } from './search';
import { api, ApiError, gatewayUrl, getToken, setToken } from './api';
import { P, has } from './perms';
import { current } from './current';
import { play, setQuiet } from './sounds';
import { notify } from './notify';

export type Status = 'online' | 'idle' | 'dnd' | 'invisible';
export type Presence = 'online' | 'idle' | 'dnd' | 'offline';

export function presenceOf(u: User | null | undefined): Presence {
  return u?.presence ?? (u?.online ? 'online' : 'offline');
}

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
  presence?: Presence;
  status?: Status;
  email?: string;
  email_verified?: boolean;
  is_admin?: boolean;
  voice?: boolean;
  two_factor?: boolean;
  badges?: Badge[];
  streaming?: boolean;
  note?: string;
  progress?: Progress;
}
export interface Progress {
  colors: string[];
  tracks: { track: string; icon: string; value: number; tier: number | null; steps: number[] }[];
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
  granted_at?: string;
  translations?: Record<string, { name: string; description: string }>;
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
  poll?: Poll;
  reply_to?: string | null;
  reply?: { id: string; deleted?: boolean; content?: string; attachments?: boolean; author?: User };
  pinned?: boolean;
  channel_name?: string;
  kind?: string;
  call?: { ended_at: string | null; participants: string[] } | null;
}
export interface Poll {
  question: string;
  multi: boolean;
  expires_at: string | null;
  options: { id: string; text: string; voters: string[] }[];
}
export type Modal =
  | { kind: 'create' ; code?: string }
  | { kind: 'guild'; tab?: string }
  | { kind: 'channel'; channel: Channel | null; parent?: string | null; type?: 'text' | 'category' | 'voice' }
  | { kind: 'user' }
  | { kind: 'groupchat'; dm: Dm | null }
  | { kind: 'mod'; userId: string }
  | null;

type Flag = boolean | (() => boolean);
export type MenuItem =
  | { label: string; hint?: string; icon?: string; danger?: boolean; disabled?: Flag; checked?: Flag; color?: string | null; keep?: boolean; action?: () => void; items?: MenuItem[]; range?: { max: number; value: () => number; set: (v: number) => void } }
  | 'sep';
export interface Menu {
  x: number;
  y: number;
  items: MenuItem[];
}

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
  menu: null as Menu | null,
  side: null as null | 'pins' | 'search',
  highlight: null as string | null,
  pinTick: 0,
  search: { query: '', total: 0, results: [] as Message[], loading: false, scope: '' },
  insert: '',
  toast: '',
});

const lastChannel: Record<string, string> = {};
let ws: WebSocket | null = null;
let lastInput = Date.now();
let idleSent = false;
const IDLE_AFTER = 10 * 60 * 1000;

function sendIdle(idle: boolean) {
  if (!ws || ws.readyState !== WebSocket.OPEN || idleSent === idle) return;
  idleSent = idle;
  ws.send(JSON.stringify({ op: 'idle', idle }));
}

function touched() {
  lastInput = Date.now();
  if (idleSent) sendIdle(false);
}

for (const ev of ['pointerdown', 'keydown', 'wheel', 'focus']) addEventListener(ev, touched, { passive: true });
let lastMove = 0;
addEventListener(
  'pointermove',
  () => {
    if (Date.now() - lastMove > 5000) {
      lastMove = Date.now();
      touched();
    }
  },
  { passive: true },
);
setInterval(() => {
  if (Date.now() - lastInput > IDLE_AFTER) sendIdle(true);
}, 30000);

export async function setStatus(status: Status) {
  const me = await attempt(api<User>('PATCH', '/users/@me', { status }));
  if (me && app.me) {
    app.me.status = status;
    setQuiet(status === 'dnd');
  }
}
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
    toast(e instanceof Error ? e.message : t('store_unknown_error'));
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
  return dm.name || dm.recipients.filter((r) => r.id !== app.me?.id).map((r) => r.display_name).join(', ') || t('store_empty_group');
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

export function findEmoji(name: string, kind: 'emoji' | 'sticker' = 'emoji', external = true) {
  const lower = name.toLowerCase();
  const own = guild()?.emojis.find((e) => e.kind === kind && e.name === lower);
  if (own || !external) return own ?? null;
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

function signedIn(res: { token?: string; two_factor?: boolean; ticket?: string }) {
  if (res.two_factor && res.ticket) return res.ticket;
  setToken(res.token!);
  app.authed = true;
  connect();
  return null;
}

export async function login(path: 'login' | 'register', body: Record<string, string>) {
  return signedIn(await api('POST', `/auth/${path}`, body));
}

export async function loginSecondFactor(ticket: string, code: string) {
  return signedIn(await api('POST', '/auth/login/2fa', { ticket, code }));
}

export async function resetPassword(token: string, password: string) {
  return signedIn(await api('POST', '/auth/reset', { token, password }));
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

function handle(t: string, d: any) {
  switch (t) {
    case 'READY': {
      retry = 0;
      app.me = d.user;
      setQuiet(d.user.status === 'dnd');
      idleSent = false;
      app.guilds = d.guilds;
      app.dms = d.dms;
      app.friends = d.friends;
      app.messages = {};
      app.more = {};
      app.connected = true;
      app.ready = true;
      if (current.voiceChannel) {
        const room = current.voiceChannel;
        const inRoom = d.guilds.some((g: Guild) => g.voice_states.some((v) => v.user_id === d.user.id && v.channel_id === room)) || d.dms.some((x: Dm) => x.id === room && x.voice.includes(d.user.id));
        (inRoom ? Promise.resolve() : api('POST', `/channels/${room}/voice`)).then(() => current.onRejoin?.()).catch(() => {});
      }
      const invite = location.hash.match(/^#\/invite\/(\w+)/);
      if (invite) {
        app.modal = { kind: 'create', code: invite[1] };
        history.replaceState(null, '', location.pathname);
      }
      const open = channel();
      if (open) {
        if (open.kind !== 'voice') loadMessages(app.channelId!);
      } else select(app.guildId && guild() ? app.guildId : (app.guilds[0]?.id ?? null));
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
          if (!isBlocked(d.author.id)) desktopNotice(d);
        }
      }
      if (app.typing[d.channel_id]) delete app.typing[d.channel_id][d.author.id];
      break;
    }
    case 'MESSAGE_UPDATE': {
      const list = app.messages[d.channel_id];
      const i = list?.findIndex((m) => m.id === d.id) ?? -1;
      if (i >= 0) list[i] = d;
      if (d.channel_id === app.channelId) app.pinTick++;
      break;
    }
    case 'MESSAGE_DELETE': {
      const list = app.messages[d.channel_id];
      if (list) {
        app.messages[d.channel_id] = list.filter((m) => m.id !== d.id);
        for (const m of app.messages[d.channel_id]) if (m.reply?.id === d.id) m.reply = { id: d.id, deleted: true };
      }
      if (d.channel_id === app.channelId) app.pinTick++;
      app.search.results = app.search.results.filter((m) => m.id !== d.id);
      break;
    }
    case 'TYPING_START':
      if (d.user_id !== app.me?.id) (app.typing[d.channel_id] ??= {})[d.user_id] = Date.now() + 6000;
      break;
    case 'PRESENCE_UPDATE':
      patchUser(d.user_id, { online: d.online, presence: d.presence, streaming: d.streaming });
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
      queueRefresh(d.guild_id, d.parts);
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

const pendingParts = new Map<string, Set<string>>();
const ALL_PARTS = ['guild', 'roles', 'channels', 'members', 'emojis'];

function queueRefresh(id: string, parts?: string[]) {
  const known = pendingParts.get(id);
  const next = known ?? new Set<string>();
  for (const p of parts?.length ? parts : ALL_PARTS) next.add(p);
  pendingParts.set(id, next);
  if (!known) setTimeout(() => refreshGuild(id), 120);
}

async function refreshGuild(id: string) {
  const parts = app.guilds.some((x) => x.id === id) ? [...(pendingParts.get(id) ?? ALL_PARTS)] : ALL_PARTS;
  pendingParts.delete(id);
  try {
    const fresh = await api<Partial<Guild> & { id: string }>('GET', `/guilds/${id}?parts=${parts.join(',')}`);
    const old = app.guilds.find((x) => x.id === id);
    putGuild((old ? { ...old, ...fresh } : fresh) as Guild);
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

export async function openDm(userId: string, show = true) {
  const dm = await attempt(api<Dm>('POST', '/users/@me/channels', { user_id: userId }));
  if (!dm) return;
  if (!app.dms.some((d) => d.id === dm.id)) app.dms.unshift(dm);
  if (show) {
    app.profile = null;
    app.modal = null;
    select(null, dm.id);
  }
  return dm;
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

export function findPerson(test: (u: User) => boolean): User | null {
  const g = guild();
  return (
    g?.members.find(test) ??
    app.guilds.flatMap((x) => x.members).find(test) ??
    app.dms.flatMap((d) => d.recipients ?? []).find(test) ??
    (app.me && test(app.me) ? app.me : null)
  );
}

function escapeRegex(text: string) {
  return text.replace(/[.*+?^${}()|[\]\\-]/g, '\\$&');
}

export function encodeMentions(text: string, people: User[]) {
  const sorted = [...people].sort((a, b) => b.username.length - a.username.length);
  let out = text;
  for (const u of sorted) {
    const re = new RegExp(`(^|[^\\w@<])@${escapeRegex(u.username)}(?![\\w.-])`, 'gi');
    out = out.replace(re, (_, pre) => `${pre}<@${u.id}>`);
  }
  return out;
}

export function decodeMentions(text: string) {
  return text.replace(/<@([0-9a-f-]{36})>/gi, (all, id) => {
    const u = findPerson((x) => x.id === id.toLowerCase());
    return u ? `@${u.username}` : all;
  });
}

export function plainText(text: string) {
  return text
    .replace(/<@([0-9a-f-]{36})>/gi, (all, id) => {
      const u = findPerson((x) => x.id === id.toLowerCase());
      return u ? `@${u.display_name}` : '@?';
    })
    .replace(/<:([a-z0-9_]{2,32}):[a-z0-9]+\.[a-z0-9]+>/gi, ':$1:');
}

export function mentionsMe(m: Pick<Message, 'content' | 'reply'>, inGuild: boolean) {
  const me = app.me;
  if (!me) return false;
  if (m.reply?.author?.id === me.id) return true;
  if (inGuild && /(^|[^\w@])@everyone(?![\w.])/i.test(m.content)) return true;
  if (m.content.toLowerCase().includes(`<@${me.id}>`)) return true;
  return new RegExp(`(^|[^\\w@])@${escapeRegex(me.username)}(?![\\w.-])`, 'i').test(m.content);
}

function desktopNotice(m: Message) {
  if (m.kind === 'call') return;
  if (app.me?.status === 'dnd') return;
  const dm = app.dms.find((x) => x.id === m.channel_id);
  const g = dm ? null : app.guilds.find((x) => x.channels.some((c) => c.id === m.channel_id));
  if (!dm && !(g && mentionsMe(m, true))) return;
  const author = g ? memberName(g, m.author) : m.author.display_name;
  const where = dm ? (dm.kind === 'group' ? dmTitle(dm) : '') : `#${g!.channels.find((c) => c.id === m.channel_id)?.name ?? ''}`;
  const title = where ? `${author} (${where})` : author;
  const body = m.content.match(/^<sticker:/) ? t('common_stickers') : m.content ? plainText(m.content).slice(0, 160) : m.poll ? m.poll.question : t('common_file');
  notify(title, body, m.channel_id, () => jumpTo(g?.id ?? null, m.channel_id, m.id));
}

export async function jumpTo(guildId: string | null, channelId: string, messageId: string) {
  app.menu = null;
  if (app.channelId !== channelId || app.guildId !== guildId) select(guildId, channelId);
  app.pane = 'chat';
  for (let i = 0; i < 25; i++) {
    if (!app.messages[channelId]) await loadMessages(channelId);
    const list = app.messages[channelId];
    if (!list || list.some((m) => m.id === messageId) || !app.more[channelId]) break;
    await loadMessages(channelId, true);
  }
  if (app.messages[channelId]?.some((m) => m.id === messageId)) app.highlight = messageId;
  else toast(t('store_message_could_not_found'));
}

export function searchPeople(): User[] {
  const g = guild();
  if (g) return g.members;
  const ch = channel();
  return ch && (ch.kind === 'dm' || ch.kind === 'group') ? (ch as Dm).recipients : [];
}

export async function runSearch(query: string, more = false) {
  const ch = channel();
  const g = guild();
  if (!ch && !g) return;
  const params = toParams(query, g, searchPeople());
  if (!params.has('channel_id')) params.set(g ? 'guild_id' : 'channel_id', g ? g.id : ch!.id);
  if (more) params.set('offset', String(app.search.results.length));
  app.side = 'search';
  app.pane = 'members';
  if (more) app.search.loading = true;
  else app.search = { query, total: 0, results: [], loading: true, scope: g ? g.id : ch!.id };
  const res = await attempt(api<{ total: number; results: Message[] }>('GET', `/search?${params}`));
  if (app.search.query !== query) return;
  const results = more ? [...app.search.results, ...(res?.results ?? []).filter((m) => !app.search.results.some((x) => x.id === m.id))] : (res?.results ?? []);
  app.search = { query, total: res?.total ?? 0, results, loading: false, scope: app.search.scope };
}
