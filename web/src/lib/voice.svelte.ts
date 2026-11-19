import { t } from './i18n.svelte';
import { api, ApiError } from './api';
import { current } from './current';
import { play } from './sounds';
import { app, toast } from './store.svelte';
import { ask } from './dialog.svelte';
import { openExternal, webAddress } from './external';

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
  streamVolume: load<Record<string, number>>('hearth_stream_volume', {}),
  userVolume: load<Record<string, number>>('hearth_user_volume', {}),
  inputVolume: load<number>('hearth_input_volume', 1),
  outputVolume: load<number>('hearth_output_volume', 1),
  watchers: {} as Record<string, string[]>,
  names: {} as Record<string, string>,
});

function load<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return v === null ? fallback : JSON.parse(v);
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}

const clamp = (v: number, max: number) => Math.min(max, Math.max(0, Number.isFinite(v) ? v : 1));

function applyVolumes() {
  if (!room || !lk) return;
  const out = voice.outputVolume;
  for (const p of room.remoteParticipants.values()) {
    p.setVolume((voice.userVolume[p.identity] ?? 1) * out, lk.Track.Source.Microphone);
    p.setVolume((voice.streamVolume[p.identity] ?? 1) * out, lk.Track.Source.ScreenShareAudio);
  }
}

export function setStreamVolume(userId: string, value: number) {
  voice.streamVolume = { ...voice.streamVolume, [userId]: clamp(value, 1) };
  save('hearth_stream_volume', voice.streamVolume);
  applyVolumes();
}

export function setUserVolume(userId: string, value: number) {
  const next = { ...voice.userVolume };
  if (clamp(value, 2) === 1) delete next[userId];
  else next[userId] = clamp(value, 2);
  voice.userVolume = next;
  save('hearth_user_volume', next);
  applyVolumes();
}

export function setOutputVolume(value: number) {
  voice.outputVolume = clamp(value, 2);
  save('hearth_output_volume', voice.outputVolume);
  applyVolumes();
}

let micGain: GainNode | null = null;

function gainProcessor() {
  let parts: { source: MediaStreamAudioSourceNode; gain: GainNode } | null = null;
  const drop = () => {
    if (!parts) return;
    parts.source.disconnect();
    parts.gain.disconnect();
    if (micGain === parts.gain) micGain = null;
    parts = null;
  };
  const build = (o: { track: MediaStreamTrack; audioContext: AudioContext }) => {
    const ctx = o.audioContext;
    const source = ctx.createMediaStreamSource(new MediaStream([o.track]));
    const gain = ctx.createGain();
    gain.gain.value = voice.inputVolume;
    const dest = ctx.createMediaStreamDestination();
    source.connect(gain).connect(dest);
    parts = { source, gain };
    micGain = gain;
    proc.processedTrack = dest.stream.getAudioTracks()[0];
  };
  const proc = {
    name: 'hearth-gain',
    processedTrack: undefined as MediaStreamTrack | undefined,
    init: async (o: any) => build(o),
    restart: async (o: any) => {
      drop();
      build(o);
    },
    destroy: async () => {
      drop();
      proc.processedTrack = undefined;
    },
  };
  return proc;
}

function micTrack() {
  if (!room || !lk) return null;
  return room.localParticipant.getTrackPublication(lk.Track.Source.Microphone)?.track ?? null;
}

async function boostMic(track: any) {
  if (!track || micGain || voice.inputVolume === 1) return;
  await track.setProcessor(gainProcessor()).catch(() => {});
}

export function setInputVolume(value: number) {
  voice.inputVolume = clamp(value, 2);
  save('hearth_input_volume', voice.inputVolume);
  if (micGain) micGain.gain.setTargetAtTime(voice.inputVolume, micGain.context.currentTime, 0.05);
  else boostMic(micTrack());
}

export const canCall = typeof window !== 'undefined' && typeof (window as any).RTCPeerConnection === 'function';

export const canShareScreen = canCall && typeof navigator !== 'undefined' && !!navigator.mediaDevices && 'getDisplayMedia' in navigator.mediaDevices;

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
  stopDesktopAudio();
  room = null;
  stopMeters();
  micGain = null;
  document.querySelectorAll('audio[data-hearth-voice]').forEach((el) => el.remove());
  Object.assign(voice, { channelId: null, guildId: null, connecting: false, muted: true, sharing: false, speaking: [], silent: [], tiles: [], streams: [], watchers: {}, names: {} });
}

export async function joinVoice(ch: { id: string; guild_id?: string | null }) {
  if (voice.connecting || voice.channelId === ch.id) return;
  if (!canCall) {
    if (await ask({ title: t('voice_no_calls_here'), text: t('voice_no_calls_here_text'), confirm: t('voice_open_in_browser') })) openExternal(webAddress());
    return;
  }
  if (voice.channelId) await leaveVoice();
  voice.connecting = true;
  try {
    const res = await api('POST', `/channels/${ch.id}/voice`);
    lk = await import('livekit-client');
    const r = new lk.Room({ adaptiveStream: true, dynacast: true, webAudioMix: true });
    room = r;
    const E = lk.RoomEvent;
    r.on(E.LocalTrackUnpublished, () => setTimeout(() => room === r && api('POST', '/voice/stream', { on: r.localParticipant.isScreenShareEnabled }).catch(() => {}), 300));
    r.on(E.TrackPublished, (pub: any) => {
      follow(pub);
      sync();
    });
    for (const ev of [E.ParticipantConnected, E.ParticipantDisconnected, E.TrackUnpublished, E.TrackSubscriptionStatusChanged, E.TrackMuted, E.TrackUnmuted, E.LocalTrackPublished, E.LocalTrackUnpublished]) r.on(ev, sync);
    r.on(E.LocalTrackPublished, (pub: any) => {
      meterMic(pub, r.localParticipant.identity);
      if (pub.source === lk!.Track.Source.Microphone) boostMic(pub.track);
    });
    r.on(E.ParticipantConnected, applyVolumes);
    r.on(E.LocalTrackUnpublished, (pub: any) => {
      if (pub.source === lk!.Track.Source.Microphone) unmeter(r.localParticipant.identity);
      if (pub.source === lk!.Track.Source.ScreenShare) stopDesktopAudio();
    });
    r.on(E.ParticipantDisconnected, (p: any) => unmeter(p.identity));
    r.on(E.TrackSubscribed, (track: any, pub: any, p: any) => {
      meterMic(pub, p.identity);
      if (track.kind === 'audio') {
        applyVolumes();
        const el = track.attach() as HTMLMediaElement;
        el.dataset.hearthVoice = '1';
        el.dataset.user = p.identity;
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
    applyVolumes();
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

const desktop = typeof window !== 'undefined' ? (window as any).hearthDesktop : null;
let shareAudio: any = null;

async function startDesktopAudio() {
  if (!room || !lk || !desktop || !(await desktop.audioShareAvailable().catch(() => false))) return;
  if (!(await desktop.audioShare(true).catch(() => false))) return;
  for (let i = 0; i < 15; i++) {
    const device = (await navigator.mediaDevices.enumerateDevices()).find((d) => d.kind === 'audioinput' && /vencord-screen-share/i.test(d.label));
    if (device) {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: { deviceId: { exact: device.deviceId }, echoCancellation: false, noiseSuppression: false, autoGainControl: false, channelCount: 2 } });
        const track = new lk.LocalAudioTrack(stream.getAudioTracks()[0], undefined, false);
        await room.localParticipant.publishTrack(track, { source: lk.Track.Source.ScreenShareAudio, audioPreset: lk.AudioPresets.musicHighQualityStereo, dtx: false, red: false });
        shareAudio = track;
        return;
      } catch {
        break;
      }
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  desktop.audioShare(false).catch(() => {});
}

async function stopDesktopAudio() {
  const track = shareAudio;
  shareAudio = null;
  if (track) {
    await room?.localParticipant.unpublishTrack(track).catch(() => {});
    track.stop();
  }
  desktop?.audioShare(false).catch(() => {});
}

export async function toggleShare() {
  if (!room || !voice.canStream) return;
  try {
    const start = !voice.sharing;
    if (!start) await stopDesktopAudio();
    await room.localParticipant.setScreenShareEnabled(
      start,
      {
        audio: desktop ? false : { echoCancellation: false, noiseSuppression: false, autoGainControl: false, restrictOwnAudio: true, suppressLocalAudioPlayback: false } as MediaTrackConstraints,
        systemAudio: 'include',
        selfBrowserSurface: 'exclude',
        surfaceSwitching: 'include',
        contentHint: 'detail',
      },
      { audioPreset: lk!.AudioPresets.musicHighQualityStereo, dtx: false, red: false },
    );
    if (start && room.localParticipant.isScreenShareEnabled && desktop) await startDesktopAudio();
    if (start && room.localParticipant.isScreenShareEnabled) {
      const sound = [...room.localParticipant.trackPublications.values()].some((pub: any) => pub.source === lk!.Track.Source.ScreenShareAudio);
      if (!sound) toast(t(desktop ? 'voice_share_no_audio_pipewire' : /Linux/i.test(navigator.userAgent) && !/Android/i.test(navigator.userAgent) ? 'voice_share_no_audio_linux' : 'voice_share_no_audio'));
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
