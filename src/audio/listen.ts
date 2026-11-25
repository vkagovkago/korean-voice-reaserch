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
