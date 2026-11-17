type Sound = 'message' | 'join' | 'leave' | 'mute' | 'unmute' | 'ring' | 'streamStart' | 'streamStop' | 'viewerJoin' | 'viewerLeave';

let ctx: AudioContext | null = null;
let ringTimer: ReturnType<typeof setInterval> | undefined;

let quiet = false;

export function setQuiet(on: boolean) {
  quiet = on;
}

export const sounds = {
  enabled: () => localStorage.getItem('hearth_sounds') !== 'off',
  set: (on: boolean) => localStorage.setItem('hearth_sounds', on ? 'on' : 'off'),
};

function unlock() {
  try {
    ctx ??= new AudioContext();
    if (ctx.state === 'suspended') ctx.resume();
  } catch {
    ctx = null;
  }
}

addEventListener('pointerdown', unlock);
addEventListener('keydown', unlock);

function tone(freq: number, at: number, length: number, volume = 0.07, type: OscillatorType = 'sine') {
  if (!ctx) return;
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  const start = ctx.currentTime + at;
  osc.type = type;
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0, start);
  gain.gain.linearRampToValueAtTime(volume, start + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
  osc.connect(gain).connect(ctx.destination);
  osc.start(start);
  osc.stop(start + length + 0.02);
}

const NOTES: Record<Sound, [number, number, number][]> = {
  message: [[880, 0, 0.12], [1175, 0.09, 0.16]],
  join: [[523, 0, 0.12], [784, 0.1, 0.18]],
  leave: [[784, 0, 0.12], [440, 0.1, 0.2]],
  mute: [[330, 0, 0.09]],
  unmute: [[494, 0, 0.09]],
  ring: [[659, 0, 0.22], [784, 0.25, 0.22], [988, 0.5, 0.3]],
  streamStart: [[587, 0, 0.1], [740, 0.08, 0.1], [880, 0.16, 0.2]],
  streamStop: [[880, 0, 0.1], [740, 0.08, 0.1], [587, 0.16, 0.2]],
  viewerJoin: [[698, 0, 0.08], [932, 0.07, 0.12]],
  viewerLeave: [[932, 0, 0.08], [698, 0.07, 0.12]],
};

export function play(name: Sound) {
  if (!sounds.enabled() || !ctx || ctx.state !== 'running') return;
  if (quiet && (name === 'message' || name === 'ring')) return;
  for (const [freq, at, length] of NOTES[name]) tone(freq, at, length);
}

export function startRing() {
  stopRing();
  play('ring');
  ringTimer = setInterval(() => play('ring'), 2200);
}

export function stopRing() {
  clearInterval(ringTimer);
  ringTimer = undefined;
}
