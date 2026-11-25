/**
 * The mic mirror: speak, and the model's folds and tongue follow. Each animation frame reads the
 * mic (`src/audio/listen.ts`): its pitch drives the folds (and whether they vibrate at all), its
 * F1/F2 the tongue, jaw and lips (the calibrated vowels nearest to them, `articForFormants`), its
 * loudness the breath. Consonants aren't followed: a stop's closure can't be read off F1/F2.
 *
 * The voice is silent unless `echo` is on, so the speakers don't feed the mic. Nothing leaves the
 * device.
 */
import { hear } from "../audio/listen.ts";
import { engine } from "../audio/AudioEngine.ts";
import { useStore } from "./store.ts";
import { getModel } from "./useModel.ts";
import { defaultF0 } from "./body.ts";
import type { HumanArtic } from "../wasm/types.ts";

/** louder than this (RMS) is a voice, quieter is a pause */
const SPEAKING = 0.01;
/** each frame moves the articulators this much of the way to the heard target */
const FOLLOW = 0.35;
/** a speaker-normalisation time constant (s): how fast the mirror learns a voice's pitch */
const LEARN = 4;
/**
 * Formants scale with the tract, pitch with the folds: a woman's formants run ~15-20% above a
 * man's where her pitch runs ~80% above, so formants go as pitch to the 0.27 (log 1.17 / log 1.8).
 * ponytail: a pitch-based guess at the speaker's tract; an F3-based one if voices of one pitch
 * read off-centre
 */
const FORMANT_PER_PITCH = 0.27;

interface Running {
  ctx: AudioContext;
  stream: MediaStream;
  analyser: AnalyserNode;
  buf: Float32Array<ArrayBuffer>;
  raf: number;
  last: number;
  /** the speaker's mean log pitch, and how many seconds of voice it's based on */
  logF0: number;
  heardFor: number;
  artic: Partial<HumanArtic>;
}

let run: Running | null = null;
let echo = false;

export const mirroring = (): boolean => run !== null;

/** Hear the mirror's voice (off by default: without headphones the speakers reach the mic). */
export function setMirrorEcho(on: boolean): void {
  echo = on;
  if (run) engine.setVolume(echo ? useStore.getState().volume : 0);
}

/** Start listening; false if the mic or the audio can't start (permission refused, no device). */
export async function startMirror(): Promise<boolean> {
  if (run) return true;
  await useStore.getState().start();
  if (!useStore.getState().started) return false;
  let stream: MediaStream;
  try {
    stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: false, autoGainControl: false } });
  } catch {
    return false;
  }
  const ctx = new AudioContext();
  const analyser = ctx.createAnalyser();
  analyser.fftSize = 2048;
  ctx.createMediaStreamSource(stream).connect(analyser);
  run = { ctx, stream, analyser, buf: new Float32Array(analyser.fftSize), raf: 0, last: performance.now(), logF0: 0, heardFor: 0, artic: {} };
  engine.setVolume(echo ? useStore.getState().volume : 0);
  useStore.getState().setSource({ voiced: true });
  run.raf = requestAnimationFrame(step);
  return true;
}

export function stopMirror(): void {
  if (!run) return;
  cancelAnimationFrame(run.raf);
  for (const t of run.stream.getTracks()) t.stop();
  void run.ctx.close();
  run = null;
  const s = useStore.getState();
  engine.gate(s.hold ? 1 : 0);
  engine.setVolume(s.volume);
  s.setSource({ voiced: true });
}

function step(now: number): void {
  if (!run) return;
  const r = run;
  r.raf = requestAnimationFrame(step);
  const dt = Math.min(0.1, (now - r.last) / 1000);
  r.last = now;
  r.analyser.getFloatTimeDomainData(r.buf);
  const h = hear(r.buf, r.ctx.sampleRate);
  const s = useStore.getState();
  if (h.rms < SPEAKING) {
    engine.gate(0);
    return;
  }
  engine.gate(1);
  // a pitch: the folds vibrate at it; none (a fricative, a whisper): they stay apart
  if (h.f0 > 0) s.setSource({ f0: Math.min(1000, Math.max(60, h.f0)), voiced: true });
  else s.setSource({ voiced: false });
  if (h.formants.length < 2 || h.f0 <= 0) return;
  const model = getModel(s.body, engine.sampleRate);
  // speaker normalisation: the heard formants scaled from this voice's tract to the model body's,
  // the tracts guessed from the voice's running pitch against the body's speaking pitch
  const w = r.heardFor === 0 ? 1 : Math.min(1, dt / LEARN);
  r.logF0 += w * (Math.log(h.f0) - r.logF0);
  r.heardFor += dt;
  const scale = Math.exp(Math.max(-0.25, Math.min(0.25, FORMANT_PER_PITCH * (Math.log(defaultF0(s.body)) - r.logF0))));
  const target = model.articForFormants(h.formants[0] * scale, h.formants[1] * scale);
  const next: Partial<HumanArtic> = {};
  for (const k of Object.keys(target) as (keyof HumanArtic)[]) {
    const prev = r.artic[k] ?? s.artic[k];
    next[k] = prev + FOLLOW * ((target[k] as number) - prev);
  }
  r.artic = next;
  s.setArtic(next);
}
