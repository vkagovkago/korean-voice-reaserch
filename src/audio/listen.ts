/**
 * Listening to a real voice, one frame at a time: its loudness, its pitch (YIN) and its first two
 * formants (LPC). Pure functions, no audio graph: the mic mirror (`src/ui/mirror.ts`) feeds them
 * an AnalyserNode's frames on the main thread, never the audio thread.
 */

export interface Heard {
  /** RMS of the frame (full scale 1) */
  rms: number;
  /** fundamental (Hz), 0 when the frame isn't voiced */
  f0: number;
  /** F1 and F2 (Hz), empty when they can't be told */
  formants: number[];
}

/** analysis rate after decimation: LPC wants ~5 kHz of bandwidth, YIN a short frame */
const TARGET_FS = 11025;

/** Average every k samples: a crude low-pass and a k-fold decimation. */
function decimate(x: ArrayLike<number>, k: number): Float64Array {
  const n = Math.floor(x.length / k);
  const y = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    let s = 0;
    for (let j = 0; j < k; j++) s += x[i * k + j];
    y[i] = s / k;
  }
  return y;
}

/** YIN (de Cheveigné & Kawahara 2002) over 60-700 Hz; 0 if no period is clear. */
export function yinF0(y: Float64Array, fs: number, threshold = 0.15): number {
  const tauMin = Math.max(2, Math.floor(fs / 700));
  const tauMax = Math.min(Math.floor(fs / 60), Math.floor(y.length / 2));
  const w = y.length - tauMax;
  if (w <= 0 || tauMax <= tauMin) return 0;
  const d = new Float64Array(tauMax + 1);
  for (let tau = 1; tau <= tauMax; tau++) {
    let s = 0;
    for (let i = 0; i < w; i++) {
      const e = y[i] - y[i + tau];
      s += e * e;
    }
    d[tau] = s;
  }
  // cumulative mean normalised difference
  let run = 0;
  let best = -1;
  const cm = new Float64Array(tauMax + 1);
  for (let tau = 1; tau <= tauMax; tau++) {
    run += d[tau];
    cm[tau] = run > 0 ? (d[tau] * tau) / run : 1;
  }
  // the first dip that is both under the threshold and close to the deepest one: a strong second
  // harmonic (F1 on 2 f0) dips under the threshold at half the period, but not as deep
  let deepest = Infinity;
  for (let tau = tauMin; tau <= tauMax; tau++) deepest = Math.min(deepest, cm[tau]);
  for (let tau = tauMin; tau <= tauMax; tau++) {
    if (cm[tau] < threshold && cm[tau] < deepest + 0.05) {
      while (tau + 1 <= tauMax && cm[tau + 1] < cm[tau]) tau++;
      best = tau;
      break;
    }
  }
  if (best < 0) return 0;
  // parabolic interpolation around the dip
  const a = cm[best - 1] ?? cm[best], b = cm[best], c = cm[best + 1] ?? cm[best];
  const den = a - 2 * b + c;
  const shift = den !== 0 ? (0.5 * (a - c)) / den : 0;
  return fs / (best + Math.max(-1, Math.min(1, shift)));
}

/** LPC coefficients a[0..p] (a[0] = 1) by autocorrelation and Levinson-Durbin. */
function lpc(x: Float64Array, p: number): Float64Array {
  const r = new Float64Array(p + 1);
  for (let k = 0; k <= p; k++) {
    let s = 0;
    for (let i = k; i < x.length; i++) s += x[i] * x[i - k];
    r[k] = s;
  }
  r[0] *= 1 + 1e-9; // a touch of white noise keeps the recursion stable
  const a = new Float64Array(p + 1);
  a[0] = 1;
  let e = r[0];
  for (let i = 1; i <= p && e > 0; i++) {
    let acc = r[i];
    for (let j = 1; j < i; j++) acc += a[j] * r[i - j];
    const k = -acc / e;
    const prev = a.slice();
    for (let j = 1; j < i; j++) a[j] = prev[j] + k * prev[i - j];
    a[i] = k;
    e *= 1 - k * k;
  }
  return a;
}

/** Roots of z^p + a1 z^(p-1) + … + ap (Durand-Kerner), as [re, im] pairs. */
function roots(a: Float64Array): [number, number][] {
  const p = a.length - 1;
  const re = new Float64Array(p), im = new Float64Array(p);
  for (let i = 0; i < p; i++) {
    // (0.4 + 0.9i)^i: the usual start, spread around the unit circle
    const m = Math.pow(Math.hypot(0.4, 0.9), i), t = i * Math.atan2(0.9, 0.4);
    re[i] = m * Math.cos(t);
    im[i] = m * Math.sin(t);
  }
  for (let it = 0; it < 300; it++) {
    let moved = 0;
    for (let i = 0; i < p; i++) {
      // P(z) by Horner
      let pr = 1, pi = 0;
      for (let k = 1; k <= p; k++) {
        const nr = pr * re[i] - pi * im[i] + a[k];
        pi = pr * im[i] + pi * re[i];
        pr = nr;
      }
      // ∏ (z_i - z_j)
      let qr = 1, qi = 0;
      for (let j = 0; j < p; j++) {
        if (j === i) continue;
        const dr = re[i] - re[j], di = im[i] - im[j];
        const nr = qr * dr - qi * di;
        qi = qr * di + qi * dr;
        qr = nr;
      }
      const den = qr * qr + qi * qi || 1e-30;
      const sr = (pr * qr + pi * qi) / den, si = (pi * qr - pr * qi) / den;
      re[i] -= sr;
      im[i] -= si;
      moved = Math.max(moved, Math.abs(sr) + Math.abs(si));
    }
    if (moved < 1e-10) break;
  }
  return Array.from(re, (r, i) => [r, im[i]] as [number, number]);
}

/**
 * Formant candidates (Hz), ascending: LPC poles with narrow bandwidths.
 * ponytail: plain autocorrelation LPC, so a low F1 under a sparse harmonic comb is pulled toward
 * the nearest harmonic (synthetic [u] at 125 Hz: 330 read 388; [i] at 230 Hz: 350 read 438). A
 * Gaussian lag window and orders 10-16 didn't fix it; a cepstrally smoothed or pitch-synchronous
 * fit would, if the mirror's close vowels read too open.
 */
export function lpcFormants(y: Float64Array, fs: number): number[] {
  const n = y.length;
  const x = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const pre = y[i] - 0.97 * (i > 0 ? y[i - 1] : 0);
    x[i] = pre * (0.54 - 0.46 * Math.cos((2 * Math.PI * i) / (n - 1)));
  }
  const p = 2 + Math.round(fs / 1000);
  const out: number[] = [];
  for (const [r, i] of roots(lpc(x, p))) {
    if (i <= 0) continue;
    const f = (Math.atan2(i, r) * fs) / (2 * Math.PI);
    const bw = (-Math.log(Math.hypot(r, i)) * fs) / Math.PI;
    if (f > 90 && f < fs / 2 - 50 && bw < 500) out.push(f);
  }
  return out.sort((p1, p2) => p1 - p2);
}

/** F1 in 200-1100 Hz, then F2 at least 150 Hz above it and under 3200. */
function firstTwo(cands: number[]): number[] {
  const f1 = cands.find((f) => f >= 200 && f <= 1100);
  if (f1 === undefined) return [];
  const f2 = cands.find((f) => f > f1 + 150 && f >= 550 && f <= 3200);
  return f2 === undefined ? [] : [f1, f2];
}

/**
 * One frame of a voice (any sample rate): loudness, pitch and F1/F2. Pitch and formants are
 * only read from a frame loud enough to hold a voice (`minRms`).
 */
export function hear(frame: ArrayLike<number>, fs: number, minRms = 0.004): Heard {
  let s = 0;
  for (let i = 0; i < frame.length; i++) s += frame[i] * frame[i];
  const rms = Math.sqrt(s / Math.max(1, frame.length));
  if (rms < minRms) return { rms, f0: 0, formants: [] };
  const k = Math.max(1, Math.round(fs / TARGET_FS));
  const y = decimate(frame, k);
  const fsd = fs / k;
  const f0 = yinF0(y, fsd);
  if (f0 <= 0) return { rms, f0: 0, formants: [] };
  // the whole frame (~46 ms at 2048 samples): a short window lets a high voice's harmonics pull
  // LPC's F1 up (a woman's [i] read 446 Hz for 350 over 35 ms)
  return { rms, f0, formants: firstTwo(lpcFormants(y, fsd)) };
}

/** Power spectrum (n a power of two) of x[start..start+n), Hann-windowed; bins 0..n/2. */
export function powerSpectrum(x: ArrayLike<number>, start: number, n: number): Float64Array {
  const re = new Float64Array(n), im = new Float64Array(n);
  for (let i = 0; i < n; i++) re[i] = (x[start + i] ?? 0) * (0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (n - 1)));
  // iterative radix-2 FFT
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) {
      [re[i], re[j]] = [re[j], re[i]];
      [im[i], im[j]] = [im[j], im[i]];
    }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const ang = (-2 * Math.PI) / len;
    for (let i = 0; i < n; i += len) {
      for (let k = 0; k < len / 2; k++) {
        const wr = Math.cos(ang * k), wi = Math.sin(ang * k);
        const ar = re[i + k + len / 2] * wr - im[i + k + len / 2] * wi;
        const ai = re[i + k + len / 2] * wi + im[i + k + len / 2] * wr;
        re[i + k + len / 2] = re[i + k] - ar;
        im[i + k + len / 2] = im[i + k] - ai;
        re[i + k] += ar;
        im[i + k] += ai;
      }
    }
  }
  const p = new Float64Array(n / 2 + 1);
  for (let k = 0; k <= n / 2; k++) p[k] = re[k] * re[k] + im[k] * im[k];
  return p;
}

export interface Shape {
  /** spectral centroid over 500-10000 Hz (Hz) */
  centroid: number;
  /** share of the energy above 1 kHz */
  high: number;
  /** centroid of the energy above 1 kHz (Hz) */
  centroidHigh: number;
}

/** Where a stretch's energy sits: its centroid and how much of it is above 1 kHz. */
export function shape(x: ArrayLike<number>, start: number, n: number, fs: number): Shape {
  const p = powerSpectrum(x, start, n);
  const hz = fs / n;
  let tot = 0, hi = 0, num = 0, den = 0, numH = 0;
  for (let k = 1; k < p.length; k++) {
    const f = k * hz;
    tot += p[k];
    if (f > 1000 && f <= 10000) (hi += p[k]), (numH += f * p[k]);
    if (f >= 500 && f <= 10000) (num += f * p[k]), (den += p[k]);
  }
  return { centroid: den > 0 ? num / den : 0, high: tot > 0 ? hi / tot : 0, centroidHigh: hi > 0 ? numH / hi : 0 };
}

export type StopPlace = "p" | "t" | "k";
/** what the stream tracker reports: a stop's place, or "ɾ" for a tap */
export type Event = StopPlace | "ɾ";

/** a hop under this RMS (-50 dBFS) is silence */
const FLOOR = 0.003;

/**
 * A stop's place from ~25 ms around its release, `start` 5-15 ms before it (Stevens & Blumstein's
 * burst shapes; thresholds set on the engine's own [apa ata aka], male and female, alone and in
 * running speech): the share of energy above 1 kHz is 0.008-0.02 for a labial, 0.15-0.30 for a
 * velar, 0.44-0.60 for an alveolar (split at 0.37). A burst mostly above 1 kHz but centred under 3 kHz is velar
 * too: speech's compact 1.5-3 kHz velar peak (the engine's velar is high and sparse instead).
 * ponytail: thresholds from synthetic speech and textbook bands; tune on real recordings
 */
export function burstPlace(x: ArrayLike<number>, start: number, fs: number): StopPlace {
  const { high, centroidHigh } = burstShape(x, start, fs);
  if (high < BURST_LABIAL) return "p";
  if (high >= BURST_ALVEOLAR) return centroidHigh < 3000 ? "k" : "t";
  return "k";
}

/** A burst's share of energy above 1 kHz and its centroid there, over the release's ~25 ms. */
function burstShape(x: ArrayLike<number>, start: number, fs: number): { high: number; centroidHigh: number } {
  // four 512-sample windows 5 ms apart, energy-weighted: where the release falls among them
  // (the tracker finds it within a hop or two) matters less than in any one window
  const step = Math.round(fs * 0.005);
  let tot = 0, hi = 0, numH = 0;
  for (let w = 0; w < 4; w++) {
    const p = powerSpectrum(x, start + w * step, 512);
    const hz = fs / 512;
    for (let k = 1; k < p.length; k++) {
      const f = k * hz;
      if (f > 10000) break;
      tot += p[k];
      if (f > 1000) (hi += p[k]), (numH += f * p[k]);
    }
  }
  return { high: tot > 0 ? hi / tot : 0, centroidHigh: hi > 0 ? numH / hi : 0 };
}
const BURST_LABIAL = 0.08;
const BURST_ALVEOLAR = 0.37;

export type Sound =
  | { kind: "silence" }
  | { kind: "vowel" }
  | { kind: "nasal"; place: "m" | "n" | "ŋ" }
  | { kind: "fricative"; place: "s" | "ɕ" | "h" };

/**
 * What a frame is, against `ref`, the recent vowels' RMS: a nasal is voiced, low-F1 (< 420 Hz) and
 * at least 30% quieter than the vowels, its place from the murmur's second resonance (the engine's
 * [m n ŋ]: 1258-1304, 1462-1480, 2399-2491 Hz); a fricative is voiceless and mostly above 1 kHz:
 * [h] faint (under a quarter of the vowels), [s] centred over 4.5 kHz, [ɕ] lower.
 * ponytail: a quiet [u] after a loud [a] can read as a nasal; a per-place murmur model if it does
 */
export function soundOf(frame: ArrayLike<number>, fs: number, heard: Heard, ref: number): Sound {
  if (heard.rms < 0.08 * ref || heard.rms < 0.004) return { kind: "silence" };
  if (heard.f0 > 0) {
    const [f1, f2] = heard.formants;
    if (f1 !== undefined && f1 < 420 && heard.rms < 0.7 * ref) return { kind: "nasal", place: f2 < 1370 ? "m" : f2 < 1950 ? "n" : "ŋ" };
    return { kind: "vowel" };
  }
  // judged on the latest ~21 ms alone: a frame that ends in a closure still holds the vowel before
  // it, loud enough to pass, while its tail is a silence whose faint noise is all high
  const n = Math.min(1024, 1 << Math.floor(Math.log2(frame.length)));
  let e = 0;
  for (let i = frame.length - n; i < frame.length; i++) e += frame[i] * frame[i];
  const tail = Math.sqrt(e / n);
  if (tail < 0.08 * ref) return { kind: "silence" };
  const s = shape(frame, frame.length - n, n, fs);
  if (s.high < 0.5) return { kind: "silence" };
  if (tail < 0.25 * ref) return { kind: "fricative", place: "h" };
  return { kind: "fricative", place: s.centroidHigh > 4500 ? "s" : "ɕ" };
}

/**
 * Stops, from a stream of samples: a closure (under 8% of the vowels' level for 15-250 ms; a
 * longer gap is a pause, and the vowel after it no stop) and
 * then a release, whose burst gives the place. `push` takes whatever arrived since the last call
 * and returns the stops released in it (each about 11 ms after its release, the burst's length).
 */
export class StopTracker {
  private readonly hop: number;
  private buf: number[] = [];
  /** absolute index of buf[0] */
  private base = 0;
  /** next sample index to examine (hop-aligned) */
  private next = 0;
  private ref = 0;
  private gapHops = 0;
  /** hops since one at vowel level, and whether the current gap began within 60 ms of one (a
   * closure can flicker, so its gap restarts a little after the vowel): a
   * stop closes out of a vowel, where a breath's faint onset before a phrase's first vowel doesn't */
  private loudAgo = 1e9;
  private gapAfterLoud = false;
  private pending = -1;
  /** a candidate release's first hops: their summed level, how many, and the gap before it. A
   * release leads into the vowel; a flicker inside a closure (the engine's velar has one 11 ms in)
   * falls silent again and is dropped, the gap going on */
  private after = 0;
  private afterHops = 0;
  private pendingGap = 0;
  /** the voice's return after a release: a one-pole low-pass (500 Hz) and its vowel reference;
   * voicing brings back the low band, aspiration doesn't */
  private lp = 0;
  private lpA: number;
  private refLow = 0;
  private relAt = -1;
  private relHops = 0;
  private votMs = -1;
  /**
   * A stop after a pause has no closure to hear: the release starts the sound. So after 250 ms of
   * silence, an onset is a stop if its first 15 ms (the burst) are followed within 50 ms by a dip
   * under 70% of them (the aspiration, or the gap before a tense stop's voice: one 5 ms hop in the
   * engine's 까) and then, within
   * 100 ms of the onset, by the voice (low band back: at least 70% of the energy at twice the dip,
   * or 50% at four times), and if its burst reaches above 1 kHz (share >= 0.08). [s] swells and
   * [h] stays level, so neither dips; a vowel's or nasal's start, a breath's thump before a phrase,
   * and a labial burst are all under 1 kHz, so a phrase-initial [p] isn't heard.
   * Set on the engine's 가 카 까 다 타 따 아 사 하 나 마; `init` is the onset (absolute index).
   */
  /**
   * A tap (ㄹ between vowels, [ɾ]): the voice dips briefly and deeply without falling silent, 10-35
   * ms under a quarter of the vowels (the engine's [ɾ] 20 ms at 0.08-0.23 of its vowel's peaks;
   * a nasal stays at 0.30-0.36, [l] 0.4 and up, a stop's closure goes silent), out of a vowel and
   * back to one.
   */
  private tapRun = 0;
  private tapFrom = false;
  private tapSilent = 0;
  /** hops left for the voice to come back after a dip that could be a tap */
  private tapWait = 0;
  private quietHops = 1e9;
  private init = -1;
  private initHops = 0;
  private initPeak = 0;
  private initDip = Infinity;
  private initRun = 0;
  private initDipped = false;
  /** absolute index before which no new release counts: a velar burst, a short silence, then the
   * voice's onset read as two stops */
  private refractory = 0;
  constructor(private readonly fs: number) {
    this.hop = Math.round(fs * 0.005);
    this.lpA = Math.exp((-2 * Math.PI * 500) / fs);
  }
  /**
   * The last release's voice onset time (ms): the first hop from 15 ms on whose low band is back
   * to 40% of the vowels' (the burst itself carries some); -1 until it's heard. The engine's
   * Korean tense, lax and aspirated stops read ~20, ~70 and ~70-75.
   */
  get vot(): number {
    return this.votMs;
  }
  /** the recent vowels' RMS (decays slowly) */
  get level(): number {
    return this.ref;
  }
  /** a release is heard but its burst not yet read */
  get releasing(): boolean {
    return this.pending >= 0 || this.init >= 0;
  }
  push(x: ArrayLike<number>): Event[] {
    for (let i = 0; i < x.length; i++) this.buf.push(x[i]);
    const out: Event[] = [];
    const end = this.base + this.buf.length;
    while (this.next + this.hop <= end) {
      const a = this.next - this.base;
      let s = 0, sl = 0;
      for (let i = a; i < a + this.hop; i++) {
        s += this.buf[i] * this.buf[i];
        this.lp = (1 - this.lpA) * this.buf[i] + this.lpA * this.lp;
        sl += this.lp * this.lp;
      }
      const e = Math.sqrt(s / this.hop);
      const low = Math.sqrt(sl / this.hop);
      this.refLow = low > this.refLow ? this.refLow + 0.3 * (low - this.refLow) : this.refLow * Math.exp(-0.005 / 2);
      if (this.relAt >= 0 && this.votMs < 0) {
        if (this.relHops >= 3 && low > 0.4 * this.refLow) this.votMs = this.relHops * 5;
        else if (++this.relHops > 40) this.relAt = -1;
        if (this.votMs >= 0) this.relAt = -1;
      }
      // the reference follows loud stretches fast and decays over ~2 s; under -50 dBFS is silence
      // whatever the reference (at the start it is 0, and a breath's hiss would count as loud)
      if (e > FLOOR) this.ref = e > this.ref ? this.ref + 0.3 * (e - this.ref) : this.ref * Math.exp(-0.005 / 2);
      else this.ref *= Math.exp(-0.005 / 2);
      // within a gap, anything short of a release (15% of the vowels) is still the closure: a
      // closure flickers, and a gap cut into pieces under 15 ms would hide the stop
      const quiet = e < FLOOR || e < 0.08 * this.ref;
      if (e > FLOOR && e < 0.25 * this.ref) {
        if (this.tapRun === 0) this.tapFrom = this.loudAgo <= 3;
        this.tapRun++;
        if (quiet) this.tapSilent++;
        this.tapWait = 0;
      } else {
        // the dip is over: the voice must be back (60% of the vowels) within 10 ms
        if (this.tapRun >= 2 && this.tapRun <= 7 && this.tapFrom && this.tapSilent < 3) this.tapWait = 3;
        if (this.tapWait > 0) {
          if (e >= 0.6 * this.ref) {
            out.push("ɾ");
            this.tapWait = 0;
          } else this.tapWait--;
        }
        this.tapRun = 0;
        this.tapSilent = 0;
      }
      if (this.init >= 0) {
        const frac = low / Math.max(e, 1e-9);
        const h = ++this.initHops;
        if (h <= 3) this.initPeak = Math.max(this.initPeak, e);
        else {
          if (h <= 10) {
            this.initDip = Math.min(this.initDip, e);
            this.initRun = e < 0.7 * this.initPeak ? this.initRun + 1 : 0;
            if (this.initRun >= 1) this.initDipped = true;
          }
          const dipped = this.initDipped;
          if (dipped && ((frac > 0.7 && e >= 2 * this.initDip) || (frac > 0.5 && e >= 4 * this.initDip))) {
            // the voice is back: a stop, released at the onset, if its burst reaches above 1 kHz.
            // A breath before a phrase's first vowel thumps, dips and voices just as a stop does,
            // all under 1 kHz; so does a labial burst, so a phrase-initial [p] isn't heard
            const at = Math.max(0, this.init - 2 * this.hop - this.base);
            if (burstShape(this.buf, at, this.fs).high >= BURST_LABIAL) {
              this.pending = this.init;
              this.afterHops = 4;
              this.refractory = this.next + Math.round(0.08 * this.fs);
              this.votMs = (h - 1) * 5;
            }
            this.init = -1;
          } else if (h > 20 || (h > 10 && !dipped)) this.init = -1;
        }
      } else if (!quiet && this.quietHops >= 50 && this.pending < 0) {
        this.init = this.next;
        this.initHops = 1;
        this.initPeak = e;
        this.initDip = Infinity;
        this.initRun = 0;
        this.initDipped = false;
      }
      this.quietHops = quiet ? this.quietHops + 1 : 0;
      if (this.pending >= 0 && this.afterHops < 4) {
        this.after += e;
        if (++this.afterHops === 4 && this.after / 4 < 0.15 * this.ref) {
          this.pending = -1;
          this.relAt = -1;
          this.refractory = 0;
          this.gapHops = this.pendingGap + 4;
          this.next += this.hop;
          continue;
        }
      }
      const silent = e < FLOOR || e < 0.08 * this.ref || (this.gapHops > 0 && e <= 0.15 * this.ref);
      if (silent) {
        if (this.gapHops === 0) this.gapAfterLoud = this.loudAgo <= 12;
        this.gapHops++;
      } else {
        const gapMs = this.gapHops * 5;
        if (this.gapHops > 0 && this.gapAfterLoud && gapMs >= 15 && gapMs <= 250 && e > 0.15 * this.ref && this.pending < 0 && this.next >= this.refractory) {
          this.pending = this.next;
          this.refractory = this.next + Math.round(0.08 * this.fs);
          this.after = 0;
          this.afterHops = 0;
          this.pendingGap = this.gapHops;
          this.relAt = this.next;
          this.relHops = 0;
          this.votMs = -1;
        }
        this.gapHops = 0;
      }
      this.loudAgo = e > FLOOR && e > 0.3 * this.ref ? 0 : this.loudAgo + 1;
      this.next += this.hop;
    }
    if (this.pending >= 0 && this.afterHops >= 4 && end - this.pending >= 512 + this.hop) {
      // from two hops before the first loud one: a labial's weak burst starts under the threshold
      out.push(burstPlace(this.buf, Math.max(0, this.pending - 2 * this.hop - this.base), this.fs));
      this.pending = -1;
    }
    // keep ~4096 samples of history before whatever is still to be read
    const keep = Math.min(this.next, this.pending >= 0 ? this.pending : this.next, this.init >= 0 ? this.init : this.next);
    const drop = Math.max(0, keep - this.base - 4096);
    if (drop > 0) {
      this.buf.splice(0, drop);
      this.base += drop;
    }
    return out;
  }
}

export type Laryngeal = "lax" | "aspirated" | "tense";
export type Heard2 = Sound | { kind: "stop"; place: StopPlace; laryngeal?: Laryngeal } | { kind: "tap" };

/**
 * One frame's call, as the mirror makes it: a stop once its burst is read, held `hold` seconds
 * (its aspiration is voiceless noise that would otherwise read as [s] or [h]), and nothing new
 * while a release waits for its burst; otherwise the frame's own sound, a nasal or fricative once
 * two frames agree.
 */
export class Listener {
  private readonly stops: StopTracker;
  /** seconds of the stop's hold left (negative: past it; none heard yet: -Infinity) */
  private left = -Infinity;
  private stop: StopPlace = "p";
  private laryngeal: Laryngeal | undefined;
  /** the speaker's running mean log pitch over vowels */
  private logF0 = 0;
  /** seconds of a tap's showing left: it's over before it's heard, so it's shown just after */
  private tapLeft = 0;
  constructor(private readonly fs: number, private readonly hold = 0.08) {
    this.stops = new StopTracker(fs);
  }
  get level(): number {
    return this.stops.level;
  }
  /** `fresh`: the samples since the last call; `frame`: the latest ~2048 (ending with them). */
  push(fresh: ArrayLike<number>, frame: ArrayLike<number>, heard: Heard): Heard2 {
    const events = this.stops.push(fresh);
    const found = events.filter((e): e is StopPlace => e !== "ɾ");
    if (found.length) {
      this.stop = found[found.length - 1];
      this.laryngeal = undefined;
      this.left = this.hold;
      return { kind: "stop", place: this.stop };
    }
    if (events.includes("ɾ")) this.tapLeft = 0.04;
    if (this.left > 0 || (this.laryngeal === undefined && this.left > -0.07)) {
      this.left -= fresh.length / this.fs;
      this.laryngeal ??= this.laryngealOf(heard);
      return { kind: "stop", place: this.stop, laryngeal: this.laryngeal };
    }
    if (this.stops.releasing) return this.said;
    if (this.tapLeft > 0) {
      this.tapLeft -= fresh.length / this.fs;
      return { kind: "tap" };
    }
    const now = soundOf(frame, this.fs, heard, Math.max(this.stops.level, 0.01));
    if (now.kind === "vowel" && heard.f0 > 0) this.logF0 = this.logF0 === 0 ? Math.log(heard.f0) : this.logF0 + 0.05 * (Math.log(heard.f0) - this.logF0);
    // a nasal or fricative counts once two frames running (~33 ms) agree: a tongue tip closing for
    // [t] makes a frame of [ɕ]-like noise; a real [s] or [m] lasts several
    const key = JSON.stringify(now);
    const steady = key === this.last;
    this.last = key;
    if ((now.kind === "nasal" || now.kind === "fricative") && !steady) return this.said;
    this.said = now;
    return now;
  }
  private last = "";
  private said: Heard2 = { kind: "silence" };

  /**
   * Once the voice is back after a release: tense if it came within 35 ms (Seoul 12-24, the
   * engine ~20); otherwise lax if its pitch starts 1.3 semitones or more under the speaker's mean,
   * aspirated if not (Seoul's aspirated and lax stops have merged in VOT and differ by ~5
   * semitones of onset pitch, Beckman et al. 2014; the engine's start -2.9 and +0.3).
   * ponytail: a phrase-final fall lowers an aspirated onset too; judge against the phrase's own
   * pitch track if that misreads
   */
  private laryngealOf(heard: Heard): Laryngeal | undefined {
    const vot = this.stops.vot;
    if (vot < 0) return undefined;
    if (vot < 35) return "tense";
    if (heard.f0 <= 0 || this.logF0 === 0) return undefined;
    const st = (12 * (Math.log(heard.f0) - this.logF0)) / Math.LN2;
    return st <= -1.3 ? "lax" : "aspirated";
  }
}
