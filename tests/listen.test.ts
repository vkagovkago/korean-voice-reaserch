import { describe, expect, test } from "bun:test";
import { hear } from "../src/audio/listen.ts";

/** A vowel: a pulse train at f0 through cascaded two-pole resonators (Klatt style), 2048 samples. */
function vowel(fs: number, f0: number, formants: [number, number][], n = 2048, gain = 0.2): Float32Array {
  let x = new Float64Array(n);
  const period = fs / f0;
  for (let t = 0; t < n; t += period) x[Math.floor(t)] = 1;
  for (const [f, bw] of formants) {
    const r = Math.exp((-Math.PI * bw) / fs);
    const b1 = 2 * r * Math.cos((2 * Math.PI * f) / fs), b2 = -r * r;
    const y = new Float64Array(n);
    for (let i = 0; i < n; i++) y[i] = (1 - b1 - b2) * x[i] + b1 * (y[i - 1] ?? 0) + b2 * (y[i - 2] ?? 0);
    x = y;
  }
  let peak = 0;
  for (const v of x) peak = Math.max(peak, Math.abs(v));
  return Float32Array.from(x, (v) => (v / peak) * gain);
}

describe("hearing a voice for the mic mirror", () => {
  // [a], [i], [u] as a man and a woman say them, at the two common device rates
  const cases: { name: string; f0: number; f: [number, number][] }[] = [
    { name: "a male", f0: 120, f: [[730, 90], [1090, 110], [2440, 160]] },
    { name: "i male", f0: 130, f: [[300, 60], [2250, 120], [3000, 200]] },
    { name: "u male", f0: 125, f: [[330, 70], [900, 90], [2300, 160]] },
    { name: "a female", f0: 220, f: [[850, 100], [1220, 120], [2800, 180]] },
    { name: "i female", f0: 230, f: [[350, 70], [2650, 140], [3300, 220]] },
  ];
  for (const fs of [44100, 48000]) {
    for (const c of cases) {
      test(`${c.name} at ${fs} Hz`, () => {
        const h = hear(vowel(fs, c.f0, c.f), fs);
        expect(Math.abs(h.f0 / c.f0 - 1)).toBeLessThan(0.03);
        expect(h.formants.length).toBe(2);
        // F1 within 30%: a close vowel's low F1 reads up to ~25% high under a high voice's
        // harmonics (see lpcFormants); F2 within 15%
        expect(Math.abs(h.formants[0] / c.f[0][0] - 1)).toBeLessThan(0.3);
        expect(Math.abs(h.formants[1] / c.f[1][0] - 1)).toBeLessThan(0.15);
      });
    }
  }

  test("open [a] reads well above close [i] and [u]", () => {
    const f1 = (c: (typeof cases)[number]) => hear(vowel(48000, c.f0, c.f), 48000).formants[0];
    expect(f1(cases[0])).toBeGreaterThan(1.6 * Math.max(f1(cases[1]), f1(cases[2])));
    expect(f1(cases[3])).toBeGreaterThan(1.6 * f1(cases[4]));
  });

  test("a strong second harmonic doesn't double the pitch", () => {
    // F1 right on 2 f0 (a 150 Hz [i] with F1 300): YIN's first dip under threshold was at half the
    // period, so the mirror read 300 Hz in the browser
    for (const fs of [44100, 48000]) {
      const h = hear(vowel(fs, 150, [[300, 40], [2250, 120], [3000, 200]]), fs);
      expect(Math.abs(h.f0 / 150 - 1)).toBeLessThan(0.03);
    }
  });

  test("silence and noise give no pitch", () => {
    expect(hear(new Float32Array(2048), 48000)).toEqual({ rms: 0, f0: 0, formants: [] });
    let seed = 1;
    const noise = Float32Array.from({ length: 2048 }, () => ((seed = (seed * 16807) % 2147483647) / 2147483647 - 0.5) * 0.2);
    const h = hear(noise, 48000);
    expect(h.rms).toBeGreaterThan(0.01);
    expect(h.f0).toBe(0);
  });
});
