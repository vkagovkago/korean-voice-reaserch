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

describe("Korean stops' three laryngeal types, as the mirror hears them", () => {
  const { lyricsToSyllables, planPhrases } = require("../src/wasm/engine.ts") as typeof import("../src/wasm/engine.ts");
  // the stop's vowel mid-phrase: the planner's statement-final fall (to 0.82 of the speaking pitch)
  // replaces the last vowel's onset pitch, and with it the lax/aspirated cue
  const CASES: [string, string, string][] = [
    ["아 가 아", "k", "lax"], ["아 카 아", "k", "aspirated"], ["아 까 아", "k", "tense"],
    ["아 다 아", "t", "lax"], ["아 타 아", "t", "aspirated"], ["아 따 아", "t", "tense"],
    ["아 바 아", "p", "lax"], ["아 파 아", "p", "aspirated"], ["아 빠 아", "p", "tense"],
    // affricates: a stop whose release frictes like [ɕ]
    ["아 자 아", "tɕ", "lax"], ["아 차 아", "tɕ", "aspirated"], ["아 짜 아", "tɕ", "tense"],
  ];
  for (const { fs, body } of VOICES) {
    for (const [text, place, type] of CASES) {
      test(`${text}: ${place}, ${type} (${body.female ? "female" : "male"} ${fs})`, () => {
        const segments = lyricsToSyllables(text).flatMap((s) => [...s.onset, s.nucleus, ...s.coda]);
        const plan = planPhrases([{ segments }], { body, t0: 0.2 } as Parameters<typeof planPhrases>[1]);
        const v = new WasmVoice(fs);
        v.schedule(plan.gestures, 0);
        const out = v.render(Math.round((plan.end + 0.3) * fs));
        const listener = new Listener(fs);
        const hop = Math.round(fs / 60);
        const stops: { place: string; laryngeal?: string }[] = [];
        for (let end = 2048; end <= out.length; end += hop) {
          const frame = out.subarray(end - 2048, end);
          const snd = listener.push(out.subarray(end - hop, end), frame, hear(frame, fs));
          if (snd.kind === "stop") stops.push({ place: snd.place, laryngeal: snd.laryngeal });
        }
        expect(new Set(stops.map((s) => s.place))).toEqual(new Set([place]));
        expect(stops.map((s) => s.laryngeal).filter(Boolean).at(-1)).toBe(type);
      });
    }
  }
});

describe("a stop after a pause (no closure to hear)", () => {
  // Real speech is silent before a phrase's first stop; the engine breathes in first, so the test
  // cuts a mid-phrase render at the consonant (a stop's release, a fricative's or vowel's start)
  // and puts 300 ms of silence before it
  const { lyricsToSyllables, planPhrases } = require("../src/wasm/engine.ts") as typeof import("../src/wasm/engine.ts");
  const CASES: [string, string | null][] = [
    ["가", "k"], ["카", "k"], ["까", "k"], ["다", "t"], ["타", "t"], ["따", "t"],
    // a labial burst is all under 1 kHz, like a breath's thump before a vowel: not heard here
    ["바", null], ["파", null], ["빠", null],
    ["아", null], ["사", null], ["하", null], ["나", null], ["마", null],
  ];
  for (const { fs, body } of VOICES) {
    for (const [syl, place] of CASES) {
      test(`${syl}: ${place ?? "no stop"} (${body.female ? "female" : "male"} ${fs})`, () => {
        const segments = lyricsToSyllables(`아 ${syl} 아`).flatMap((s) => [...s.onset, s.nucleus, ...s.coda]);
        const plan = planPhrases([{ segments }], { body, t0: 0.2 } as Parameters<typeof planPhrases>[1]);
        const v = new WasmVoice(fs);
        v.schedule(plan.gestures, 0);
        const out = v.render(Math.round((plan.end + 0.3) * fs));
        const seg = plan.segments[1];
        // a stop from 2 ms before its release; anything else from 15 ms in (at its very start the
        // cut would carry the preceding [a]'s tail, a burst-and-dip no real onset has)
        const from = Math.round((seg.kind === "stop" ? seg.t1 - 0.002 : seg.t0 + 0.015) * fs);
        const sig = new Float32Array(Math.round(0.3 * fs) + out.length - from);
        sig.set(out.subarray(from), Math.round(0.3 * fs));
        const listener = new Listener(fs);
        const hop = Math.round(fs / 60);
        const stops = new Set<string>();
        for (let end = 2048; end <= sig.length; end += hop) {
          const frame = sig.subarray(end - 2048, end);
          const snd = listener.push(sig.subarray(end - hop, end), frame, hear(frame, fs));
          if (snd.kind === "stop") stops.add(snd.place);
        }
        expect([...stops]).toEqual(place ? [place] : []);
      });
    }
  }
});

describe("a tap (ㄹ between vowels)", () => {
  const { lyricsToSyllables, planPhrases } = require("../src/wasm/engine.ts") as typeof import("../src/wasm/engine.ts");
  // what the mirror reports over the word: taps, stops, nasals
  const CASES: [string, string[]][] = [
    ["아라 아", ["tap"]], ["아리 아", ["tap"]], ["아나 아", ["nasal"]], ["아마 아", ["nasal"]],
    ["알라 아", []], ["아아 아", []], ["아다 아", ["stop"]],
    // no tap in a fricative's or an affricate's noise (both once read one)
    ["아 사 아", ["fricative"]], ["아 짜 아", ["stop"]],
  ];
  for (const { fs, body } of VOICES) {
    for (const [text, want] of CASES) {
      test(`${text}: ${want.join(", ") || "nothing"} (${body.female ? "female" : "male"} ${fs})`, () => {
        const segments = lyricsToSyllables(text).flatMap((s) => [...s.onset, s.nucleus, ...s.coda]);
        const plan = planPhrases([{ segments }], { body, t0: 0.2 } as Parameters<typeof planPhrases>[1]);
        const v = new WasmVoice(fs);
        v.schedule(plan.gestures, 0);
        const out = v.render(Math.round((plan.end + 0.3) * fs));
        const listener = new Listener(fs);
        const hop = Math.round(fs / 60);
        const kinds = new Set<string>();
        for (let end = 2048; end <= out.length; end += hop) {
          const frame = out.subarray(end - 2048, end);
          const k = listener.push(out.subarray(end - hop, end), frame, hear(frame, fs)).kind;
          if (k !== "vowel" && k !== "silence") kinds.add(k);
        }
        expect([...kinds].sort()).toEqual([...want].sort());
      });
    }
  }
});
