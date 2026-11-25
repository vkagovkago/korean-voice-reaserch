import { describe, expect, test } from "bun:test";
import { initEngine, WasmVoice } from "./harness.ts";
import { hear, Listener, soundOf, StopTracker, type Sound } from "../src/audio/listen.ts";

await initEngine();
const { planUtterance, tokenizeIpa } = await import("../src/wasm/engine.ts");

const VOICES = [
  { fs: 48000, body: { female: 0, child: 0 } },
  { fs: 44100, body: { female: 1, child: 0 } },
];

/** The engine saying `ipa`, and its plan's segments. */
function say(ipa: string, fs: number, body: { female: number; child: number }) {
  const plan = planUtterance(tokenizeIpa(ipa), { body, t0: 0.1 } as Parameters<typeof planUtterance>[1]);
  const v = new WasmVoice(fs);
  v.schedule(plan.gestures, 0);
  return { out: v.render(Math.round((plan.end + 0.2) * fs)), segs: plan.segments };
}

/** Feed it as the mirror does, ~one animation frame (16.7 ms) at a time. */
function stops(out: Float32Array, fs: number): string[] {
  const t = new StopTracker(fs);
  const hop = Math.round(fs / 60);
  const found: string[] = [];
  for (let i = 0; i < out.length; i += hop) found.push(...t.push(out.subarray(i, i + hop)));
  return found;
}

describe("following consonants for the mic mirror", () => {
  for (const { fs, body } of VOICES) {
    const who = `${body.female ? "female" : "male"} ${fs}`;
    for (const c of ["p", "t", "k"]) {
      test(`a${c}a: one stop, ${c} (${who})`, () => {
        const { out } = say(`a${c}a`, fs, body);
        expect(stops(out, fs)).toEqual([c]);
      });
    }
    test(`a vowel alone, and a nasal, have no stop (${who})`, () => {
      expect(stops(say("a", fs, body).out, fs)).toEqual([]);
      expect(stops(say("ana", fs, body).out, fs)).toEqual([]);
    });
    const middle: [string, Sound][] = [
      ["ama", { kind: "nasal", place: "m" }],
      ["ana", { kind: "nasal", place: "n" }],
      ["aŋa", { kind: "nasal", place: "ŋ" }],
      ["asa", { kind: "fricative", place: "s" }],
      ["aɕa", { kind: "fricative", place: "ɕ" }],
      ["aha", { kind: "fricative", place: "h" }],
    ];
    for (const [w, want] of middle) {
      test(`${w}: its middle reads ${JSON.stringify(want)} (${who})`, () => {
        const { out, segs } = say(w, fs, body);
        const ref = hear(out.subarray(Math.round((segs[0].t0 + 0.03) * fs), Math.round((segs[0].t0 + 0.03) * fs) + 2048), fs).rms;
        const mid = Math.round(0.5 * (segs[1].t0 + segs[1].t1) * fs);
        const frame = out.subarray(mid - 1024, mid + 1024);
        expect(soundOf(frame, fs, hear(frame, fs), ref)).toEqual(want);
      });
    }
    test(`vowels read as vowels (${who})`, () => {
      for (const v of ["a", "i", "u", "o", "ɯ"]) {
        const { out, segs } = say(v, fs, body);
        const mid = Math.round(0.5 * (segs[0].t0 + segs[0].t1) * fs);
        const frame = out.subarray(mid - 1024, mid + 1024);
        const h = hear(frame, fs);
        expect(soundOf(frame, fs, h, h.rms)).toEqual({ kind: "vowel" });
      }
    });
  }
});

describe("the mirror's frame loop over running speech", () => {
  // what the mirror decides at each animation frame (src/ui/mirror.ts uses the same Listener)
  for (const { fs, body } of VOICES) {
    test(`apa ata aka ama ana asa aha (${body.female ? "female" : "male"} ${fs})`, () => {
      const words = ["apa", "ata", "aka", "ama", "ana", "asa", "aha"];
      const { planPhrases } = require("../src/wasm/engine.ts") as typeof import("../src/wasm/engine.ts");
      const plan = planPhrases(words.map((w) => ({ segments: tokenizeIpa(w), pause: 0.5 })), { body, t0: 0.2 } as Parameters<typeof planPhrases>[1]);
      const v = new WasmVoice(fs);
      v.schedule(plan.gestures, 0);
      const out = v.render(Math.round((plan.end + 0.3) * fs));
      const listener = new Listener(fs);
      const hop = Math.round(fs / 60);
      const seen: { t: number; what: string }[] = [];
      for (let end = 2048; end <= out.length; end += hop) {
        const frame = out.subarray(end - 2048, end);
        const snd = listener.push(out.subarray(end - hop, end), frame, hear(frame, fs));
        if (snd.kind === "stop" || snd.kind === "nasal" || snd.kind === "fricative") seen.push({ t: end / fs, what: snd.place });
      }
      const cons = plan.segments.filter((s) => !"a".includes(s.ipa));
      expect(cons.map((s) => s.ipa)).toEqual(["p", "t", "k", "m", "n", "s", "h"]);
      for (const c of cons) {
        // heard during the consonant or up to 120 ms after it (a stop is heard at its release)
        const near = seen.filter((x) => x.t >= c.t0 && x.t <= c.t1 + 0.12).map((x) => x.what);
        expect(near).toContain(c.ipa);
        // and no other consonant there
        expect(near.filter((w) => w !== c.ipa)).toEqual([]);
      }
    });
  }
});
