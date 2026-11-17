import { t } from './i18n.svelte';
import { api, ApiError } from './api';
import { current } from './current';
import { play } from './sounds';
import { app, toast } from './store.svelte';

export interface Tile {
  id: string;
  userId: string;
  name: string;
  kind: 'screen' | 'camera';
  track: any;
  local: boolean;
}

export const voice = $state({
  channelId: null as string | null,
  guildId: null as string | null,
  connecting: false,
  muted: true,
  sharing: false,
  canSpeak: false,
  canStream: false,
  speaking: [] as string[],
  silent: [] as string[],
  tiles: [] as Tile[],
  streams: [] as { userId: string; name: string; local: boolean; watching: boolean }[],
  watchers: {} as Record<string, string[]>,
  names: {} as Record<string, string>,
});

export const canShareScreen = typeof navigator !== 'undefined' && !!navigator.mediaDevices && 'getDisplayMedia' in navigator.mediaDevices;

let room: any = null;
let lk: typeof import('livekit-client') | null = null;

function sync() {
  if (!room || !lk) return;
  const tiles: Tile[] = [];
  const silent: string[] = [];
  const streams: typeof voice.streams = [];
  const names: Record<string, string> = {};
  for (const p of [room.localParticipant, ...room.remoteParticipants.values()]) {
    const local = p === room.localParticipant;
    let mic = false;
    names[p.identity] = p.name || p.identity;
    for (const pub of p.trackPublications.values()) {
      if (pub.source === lk.Track.Source.Microphone) mic = !pub.isMuted;
      if (pub.source === lk.Track.Source.ScreenShare && !pub.isMuted) streams.push({ userId: p.identity, name: p.name || p.identity, local, watching: local || pub.isSubscribed });
      if (pub.kind === lk.Track.Kind.Video && pub.track && !pub.isMuted) {
        tiles.push({ id: pub.trackSid, userId: p.identity, name: p.name || p.identity, kind: pub.source === lk.Track.Source.ScreenShare ? 'screen' : 'camera', track: pub.track, local });
      }
    }
    if (!mic) silent.push(p.identity);
  }
  if (streams.length !== voice.streams.length) play(streams.length > voice.streams.length ? 'streamStart' : 'streamStop');
  voice.tiles = tiles;
  voice.streams = streams;
  voice.names = names;
  voice.silent = silent;
  voice.muted = !room.localParticipant.isMicrophoneEnabled;
  voice.sharing = room.localParticipant.isScreenShareEnabled;
}

function isScreen(pub: any) {
  return pub.source === lk!.Track.Source.ScreenShare || pub.source === lk!.Track.Source.ScreenShareAudio;
}

function follow(pub: any) {
  if (!isScreen(pub)) pub.setSubscribed(true);
}

function setWatchers(pairs: [string, string][], quiet = false) {
  const next: Record<string, string[]> = {};
  for (const [viewer, streamer] of pairs) (next[streamer] ??= []).push(viewer);
  const me = app.me?.id ?? '';
  if (!quiet) {
    for (const streamer of new Set([...Object.keys(next), ...Object.keys(voice.watchers)])) {
      const before = voice.watchers[streamer] ?? [];
      const after = next[streamer] ?? [];
      if (streamer !== me && !before.includes(me) && !after.includes(me)) continue;
      const others = (list: string[]) => list.filter((v) => v !== me).length;
      if (others(after) !== others(before)) play(others(after) > others(before) ? 'viewerJoin' : 'viewerLeave');
    }
  }
  voice.watchers = next;
}

current.onWatchers = (pairs) => setWatchers(pairs);

export async function watchStream(userId: string, on: boolean) {
  const p = room?.remoteParticipants.get(userId);
  if (!p) return;
  for (const pub of p.trackPublications.values()) if (isScreen(pub)) pub.setSubscribed(on);
  await api('POST', '/voice/watch', { user_id: userId, on }).catch(() => {});
  sync();
}

function cleanup() {
  if (current.voiceChannel) play('leave');
  current.voiceChannel = null;
  room = null;
  document.querySelectorAll('audio[data-hearth-voice]').forEach((el) => el.remove());
  Object.assign(voice, { channelId: null, guildId: null, connecting: false, muted: true, sharing: false, speaking: [], silent: [], tiles: [], streams: [], watchers: {}, names: {} });
}

export async function joinVoice(ch: { id: string; guild_id?: string | null }) {
  if (voice.connecting || voice.channelId === ch.id) return;
  if (voice.channelId) await leaveVoice();
  voice.connecting = true;
  try {
    const res = await api('POST', `/channels/${ch.id}/voice`);
    lk = await import('livekit-client');
    const r = new lk.Room({ adaptiveStream: true, dynacast: true });
    room = r;
    const E = lk.RoomEvent;
    r.on(E.LocalTrackUnpublished, () => setTimeout(() => room === r && api('POST', '/voice/stream', { on: r.localParticipant.isScreenShareEnabled }).catch(() => {}), 300));
    r.on(E.TrackPublished, (pub: any) => {
      follow(pub);
      sync();
    });
    for (const ev of [E.ParticipantConnected, E.ParticipantDisconnected, E.TrackUnpublished, E.TrackSubscriptionStatusChanged, E.TrackMuted, E.TrackUnmuted, E.LocalTrackPublished, E.LocalTrackUnpublished]) r.on(ev, sync);
    r.on(E.TrackSubscribed, (track: any) => {
      if (track.kind === 'audio') {
        const el = track.attach() as HTMLMediaElement;
        el.dataset.hearthVoice = '1';
        el.hidden = true;
        document.body.append(el);
      }
      sync();
    });
    r.on(E.TrackUnsubscribed, (track: any) => {
      if (track.kind === 'audio') track.detach().forEach((el: HTMLElement) => el.remove());
      sync();
    });
    r.on(E.ActiveSpeakersChanged, (speakers: any[]) => (voice.speaking = speakers.map((s) => s.identity)));
    r.on(E.Disconnected, () => {
      if (room === r) {
        cleanup();
        api('DELETE', '/voice').catch(() => {});
      }
    });
    await r.connect(res.url, res.token, { autoSubscribe: false });
    for (const p of r.remoteParticipants.values()) for (const pub of p.trackPublications.values()) follow(pub);
    setWatchers(res.watchers ?? [], true);
    current.voiceChannel = ch.id;
    play('join');
    Object.assign(voice, { channelId: ch.id, guildId: ch.guild_id ?? null, canSpeak: res.can_speak, canStream: res.can_stream });
    if (res.can_speak) await r.localParticipant.setMicrophoneEnabled(true).catch(() => toast(t('voice_no_access_microphone_listen_but')));
    sync();
  } catch (e) {
    toast(e instanceof ApiError ? e.message : t('voice_voice_server_unreachable'));
    const r = room;
    cleanup();
    r?.disconnect();
    api('DELETE', '/voice').catch(() => {});
  }
  voice.connecting = false;
}

export async function leaveVoice() {
  const r = room;
  cleanup();
  await r?.disconnect();
  await api('DELETE', '/voice').catch(() => {});
}

export async function toggleMute() {
  if (!room || !voice.canSpeak) return;
  await room.localParticipant.setMicrophoneEnabled(voice.muted).catch(() => toast(t('voice_no_access_microphone')));
  sync();
  play(voice.muted ? 'mute' : 'unmute');
}

export async function toggleShare() {
  if (!room || !voice.canStream) return;
  try {
    await room.localParticipant.setScreenShareEnabled(!voice.sharing, { audio: true });
  } catch {
    if (!voice.sharing) toast(t('voice_screen_sharing_was_cancelled_not'));
  }
  sync();
  api('POST', '/voice/stream', { on: voice.sharing }).catch(() => {});
}

export function voiceMembers(channelId: string) {
  const g = app.guilds.find((x) => x.channels.some((c) => c.id === channelId));
  if (!g) return [];
  return g.voice_states.filter((v) => v.channel_id === channelId).flatMap((v) => g.members.find((m) => m.id === v.user_id) ?? []);
}
