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
  streams: [] as { userId: string; name: string; local: boolean; watching: boolean; audio: boolean }[],
  streamVolume: loadVolumes(),
  watchers: {} as Record<string, string[]>,
  names: {} as Record<string, string>,
});

function loadVolumes(): Record<string, number> {
  try {
    return JSON.parse(localStorage.getItem('hearth_stream_volume') ?? '{}');
  } catch {
    return {};
  }
}

function applyVolume(el: HTMLMediaElement) {
  const v = voice.streamVolume[el.dataset.user ?? ''] ?? 1;
  el.volume = Math.min(1, Math.max(0, v));
  el.muted = v === 0;
}

export function setStreamVolume(userId: string, value: number) {
  voice.streamVolume = { ...voice.streamVolume, [userId]: value };
  try {
    localStorage.setItem('hearth_stream_volume', JSON.stringify(voice.streamVolume));
  } catch {}
  document.querySelectorAll<HTMLMediaElement>(`audio[data-hearth-voice][data-stream][data-user="${CSS.escape(userId)}"]`).forEach(applyVolume);
}

export const canShareScreen = typeof navigator !== 'undefined' && !!navigator.mediaDevices && 'getDisplayMedia' in navigator.mediaDevices;

let room: any = null;
let lk: typeof import('livekit-client') | null = null;

const meters = new Map<string, { track: MediaStreamTrack; source: MediaStreamAudioSourceNode; analyser: AnalyserNode }>();
const loudAt = new Map<string, number>();
let audio: AudioContext | null = null;
let ticker: ReturnType<typeof setInterval> | null = null;
const samples = new Float32Array(512);

function meter(identity: string, track: MediaStreamTrack | undefined) {
  if (!track) return;
  if (meters.get(identity)?.track === track) return;
  unmeter(identity);
  audio ??= new AudioContext();
  audio.resume().catch(() => {});
  const source = audio.createMediaStreamSource(new MediaStream([track]));
  const analyser = audio.createAnalyser();
  analyser.fftSize = 512;
  source.connect(analyser);
  meters.set(identity, { track, source, analyser });
  ticker ??= setInterval(listen, 50);
}

function unmeter(identity: string) {
  const m = meters.get(identity);
  if (!m) return;
  m.source.disconnect();
  meters.delete(identity);
  loudAt.delete(identity);
}

function listen() {
  const now = performance.now();
  for (const [identity, m] of meters) {
    m.analyser.getFloatTimeDomainData(samples);
    let sum = 0;
    for (const v of samples) sum += v * v;
    if (Math.sqrt(sum / samples.length) > 0.012) loudAt.set(identity, now);
  }
  const next = [...loudAt].filter(([identity, at]) => now - at < 280 && !voice.silent.includes(identity)).map(([identity]) => identity).sort();
  if (next.join() !== voice.speaking.join()) voice.speaking = next;
}

function stopMeters() {
  for (const identity of [...meters.keys()]) unmeter(identity);
  if (ticker) clearInterval(ticker);
  ticker = null;
  audio?.close().catch(() => {});
  audio = null;
}

function meterMic(pub: any, identity: string) {
  if (pub?.source === lk?.Track.Source.Microphone) meter(identity, pub.track?.mediaStreamTrack);
}

function sync() {
  if (!room || !lk) return;
  const tiles: Tile[] = [];
  const silent: string[] = [];
  const streams: typeof voice.streams = [];
  const names: Record<string, string> = {};
  for (const p of [room.localParticipant, ...room.remoteParticipants.values()]) {
    const local = p === room.localParticipant;
    let mic = false;
    const audio = [...p.trackPublications.values()].some((pub: any) => pub.source === lk!.Track.Source.ScreenShareAudio && !pub.isMuted);
    names[p.identity] = p.name || p.identity;
    for (const pub of p.trackPublications.values()) {
      if (pub.source === lk.Track.Source.Microphone) {
        mic = !pub.isMuted;
        if (pub.track) meter(p.identity, pub.track.mediaStreamTrack);
      }
      if (pub.source === lk.Track.Source.ScreenShare && !pub.isMuted) streams.push({ userId: p.identity, name: p.name || p.identity, local, watching: local || pub.isSubscribed, audio });
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
  stopMeters();
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
    r.on(E.LocalTrackPublished, (pub: any) => meterMic(pub, r.localParticipant.identity));
    r.on(E.LocalTrackUnpublished, (pub: any) => pub.source === lk!.Track.Source.Microphone && unmeter(r.localParticipant.identity));
    r.on(E.ParticipantDisconnected, (p: any) => unmeter(p.identity));
    r.on(E.TrackSubscribed, (track: any, pub: any, p: any) => {
      meterMic(pub, p.identity);
      if (track.kind === 'audio') {
        const el = track.attach() as HTMLMediaElement;
        el.dataset.hearthVoice = '1';
        el.dataset.user = p.identity;
        if (pub.source === lk!.Track.Source.ScreenShareAudio) {
          el.dataset.stream = '1';
          applyVolume(el);
        }
        el.hidden = true;
        document.body.append(el);
      }
      sync();
    });
    r.on(E.TrackUnsubscribed, (track: any, pub: any, p: any) => {
      if (pub.source === lk!.Track.Source.Microphone) unmeter(p.identity);
      if (track.kind === 'audio') track.detach().forEach((el: HTMLElement) => el.remove());
      sync();
    });
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
    const start = !voice.sharing;
    await room.localParticipant.setScreenShareEnabled(
      start,
      {
        audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false, restrictOwnAudio: true, suppressLocalAudioPlayback: false } as MediaTrackConstraints,
        systemAudio: 'include',
        selfBrowserSurface: 'exclude',
        surfaceSwitching: 'include',
        contentHint: 'detail',
      },
      { audioPreset: lk!.AudioPresets.musicHighQualityStereo, dtx: false, red: false },
    );
    if (start && room.localParticipant.isScreenShareEnabled) {
      const sound = [...room.localParticipant.trackPublications.values()].some((pub: any) => pub.source === lk!.Track.Source.ScreenShareAudio);
      if (!sound) toast(t(/Linux/i.test(navigator.userAgent) && !/Android/i.test(navigator.userAgent) ? 'voice_share_no_audio_linux' : 'voice_share_no_audio'));
    }
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
