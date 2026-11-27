/**
 * How the mic mirror's listener (src/audio/listen.ts) hears real speakers: each TSP recording is
 * fed to a Listener as the mirror feeds it (an animation frame's samples at a time, the latest 2048
 * for the frame), and every aligned phone gets what the listener said over it (from its start to
 * 100 ms past its end: a stop is heard at its release). Prints a confusion table per phone class.
 *
 *   bun tools/tsp/mirror-eval.ts [tsp-dir] [--speakers FA,MA,…] [--limit n]
 *
 * Needs tools/tsp/fetch.sh (the recordings) and tools/tsp/alignment.tsv.gz (in git).
 */
import { readFileSync, existsSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { homedir } from "node:os";
import { join } from "node:path";
import { hear, Listener } from "../../src/audio/listen.ts";

const args = process.argv.slice(2);
const opt = (name: string) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const dir = args.find((a, i) => !a.startsWith("--") && !args[i - 1]?.startsWith("--")) ?? join(homedir(), "datasets", "tsp");
const speakers = opt("--speakers")?.split(",");
const limit = Number(opt("--limit") ?? Infinity);

/** 16-bit PCM mono WAV → samples and rate */
function readWav(path: string): { x: Float32Array; fs: number } {
  const b = readFileSync(path);
  let p = 12, fs = 0, bits = 16, ch = 1;
  while (p + 8 <= b.length) {
    const id = b.toString("ascii", p, p + 4), n = b.readUInt32LE(p + 4);
    if (id === "fmt ") {
      ch = b.readUInt16LE(p + 10);
      fs = b.readUInt32LE(p + 12);
      bits = b.readUInt16LE(p + 22);
    } else if (id === "data") {
      if (bits !== 16) throw new Error(`${path}: ${bits}-bit`);
      const frames = Math.floor(n / 2 / ch);
      const x = new Float32Array(frames);
      for (let i = 0; i < frames; i++) x[i] = b.readInt16LE(p + 8 + i * 2 * ch) / 32768;
      return { x, fs };
    }
    p += 8 + n + (n & 1);
  }
  throw new Error(`${path}: no data`);
}

/** ARPAbet → the class the mirror could report for it */
function classOf(ph: string, final: boolean): string | null {
  const p = ph.replace(/[0-9]/g, "");
  const stop: Record<string, string> = { P: "p", B: "p", T: "t", D: "t", K: "k", G: "k" };
  if (stop[p]) return (final ? "final " : "") + stop[p];
  if (p === "CH" || p === "JH") return "tɕ";
  if (p === "M") return "m";
  if (p === "N") return "n";
  if (p === "NG") return "ŋ";
  if (p === "S" || p === "Z") return "s";
  if (p === "SH" || p === "ZH") return "ɕ";
  if (p === "HH") return "h";
  if (p === "L") return (final ? "final " : "") + "l";
  if (/^(AA|AE|AH|AO|AW|AY|EH|ER|EY|IH|IY|OW|OY|UH|UW)$/.test(p)) return "vowel";
  return null;
}

/** what the listener said, as one label */
function said(s: ReturnType<Listener["push"]>): string {
  if (s.kind === "stop") return s.place;
  if (s.kind === "nasal" || s.kind === "fricative") return s.place;
  if (s.kind === "tap") return "ɾ";
  if (s.kind === "lateral") return "l";
  return s.kind;
}
/** the most specific report wins over a phone's span */
const RANK = (l: string) => (["p", "t", "k", "tɕ"].includes(l) ? 4 : l === "ɾ" ? 3 : ["m", "n", "ŋ", "l", "s", "ɕ", "h"].includes(l) ? 2 : l === "vowel" ? 1 : 0);

const align = gunzipSync(readFileSync(new URL("./alignment.tsv.gz", import.meta.url))).toString("utf8").trim().split("\n").map((l) => l.split("\t"));
const byRec = new Map<string, { tier: string; t0: number; t1: number; label: string }[]>();
for (const [rec, tier, t0, t1, label] of align) {
  if (!byRec.has(rec)) byRec.set(rec, []);
  byRec.get(rec)!.push({ tier, t0: +t0, t1: +t1, label });
}

const table = new Map<string, Map<string, number>>();
/** what the listener said at each phone's middle frame (a sonorant or fricative's steady part) */
const midTable = new Map<string, Map<string, number>>();
let done = 0;
/** stop reports: how many, how many land on an aligned stop or affricate, with its place right */
const ev = { reports: 0, onStop: 0, placeRight: 0, stops: 0, heard: 0, seconds: 0 };
const STOP_PLACE: Record<string, string> = { P: "p", B: "p", T: "t", D: "t", K: "k", G: "k", CH: "tɕ", JH: "tɕ" };
for (const [rec, rows] of byRec) {
  const spk = rec.replace(/[0-9].*/, "");
  if (speakers && !speakers.includes(spk)) continue;
  if (done >= limit) break;
  const path = join(dir, "48k", spk, `${rec}.wav`);
  if (!existsSync(path)) continue;
  const { x, fs } = readWav(path);
  const listener = new Listener(fs);
  const hop = Math.round(fs / 60);
  const frames: { t: number; l: string }[] = [];
  for (let end = 2048; end <= x.length; end += hop) {
    const frame = x.subarray(end - 2048, end);
    frames.push({ t: end / fs, l: said(listener.push(x.subarray(end - hop, end), frame, hear(frame, fs))) });
  }
  const words = rows.filter((r) => r.tier === "word");
  const phones = rows.filter((r) => r.tier === "phone");
  const stopPhones = phones.filter((p) => STOP_PLACE[p.label.replace(/[0-9]/g, "")]);
  ev.seconds += x.length / fs;
  ev.stops += stopPhones.length;
  const isStop = (l: string) => ["p", "t", "k", "tɕ"].includes(l);
  for (let i = 0; i < frames.length; i++) {
    if (!isStop(frames[i].l) || (i > 0 && isStop(frames[i - 1].l))) continue;
    ev.reports++;
    const hit = stopPhones.find((p) => frames[i].t >= p.t0 && frames[i].t <= p.t1 + 0.08);
    if (hit) {
      ev.onStop++;
      if (STOP_PLACE[hit.label.replace(/[0-9]/g, "")] === frames[i].l) ev.placeRight++;
    }
  }
  for (const p of stopPhones) if (frames.some((f) => isStop(f.l) && f.t >= p.t0 && f.t <= p.t1 + 0.08)) ev.heard++;
  for (const ph of phones) {
    // word-final, and before a pause or the end (a coda with nothing to carry it on)
    const w = words.find((w) => w.t1 === ph.t1);
    const nextWord = words.find((v) => v.t0 >= ph.t1 - 1e-6);
    const final = !!w && (!nextWord || nextWord.t0 - ph.t1 > 0.1);
    const cls = classOf(ph.label, final);
    if (!cls) continue;
    let best = "silence";
    for (const f of frames) if (f.t >= ph.t0 && f.t <= ph.t1 + 0.1 && RANK(f.l) > RANK(best)) best = f.l;
    if (!table.has(cls)) table.set(cls, new Map());
    const row = table.get(cls)!;
    row.set(best, (row.get(best) ?? 0) + 1);
    // the frame whose window centres nearest the phone's middle (a frame covers the 2048 samples
    // before its time)
    const mid = (ph.t0 + ph.t1) / 2 + 1024 / fs;
    const near = frames.reduce((a, f) => (Math.abs(f.t - mid) < Math.abs(a.t - mid) ? f : a), frames[0]);
    if (!midTable.has(cls)) midTable.set(cls, new Map());
    const mrow = midTable.get(cls)!;
    mrow.set(near.l, (mrow.get(near.l) ?? 0) + 1);
  }
  done++;
}

const cols = ["p", "t", "k", "tɕ", "ɾ", "m", "n", "ŋ", "l", "s", "ɕ", "h", "vowel", "silence"];
console.log(`${done} recordings, ${ev.seconds.toFixed(0)} s`);
const pct = (a: number, b: number) => `${Math.round((100 * a) / Math.max(1, b))}%`;
console.log(`stop reports ${ev.reports} (${(ev.reports / ev.seconds).toFixed(1)}/s): on an aligned stop ${pct(ev.onStop, ev.reports)}, place right ${pct(ev.placeRight, ev.onStop)} of those; stops heard ${pct(ev.heard, ev.stops)} of ${ev.stops}\n`);
console.log(["phone".padEnd(9), "n".padStart(5), ...cols.map((c) => c.padStart(6))].join(""));
for (const cls of ["p", "t", "k", "final p", "final t", "final k", "tɕ", "m", "n", "ŋ", "s", "ɕ", "h", "l", "final l", "vowel"]) {
  const row = table.get(cls);
  if (!row) continue;
  const n = [...row.values()].reduce((a, b) => a + b, 0);
  console.log([cls.padEnd(9), String(n).padStart(5), ...cols.map((c) => `${Math.round((100 * (row.get(c) ?? 0)) / n)}%`.padStart(6))].join(""));
}

console.log("\nat each phone's middle frame:");
console.log(["phone".padEnd(9), "n".padStart(5), ...cols.map((c) => c.padStart(6))].join(""));
for (const cls of ["m", "n", "ŋ", "l", "final l", "s", "ɕ", "h", "vowel"]) {
  const row = midTable.get(cls);
  if (!row) continue;
  const n = [...row.values()].reduce((a, b) => a + b, 0);
  console.log([cls.padEnd(9), String(n).padStart(5), ...cols.map((c) => `${Math.round((100 * (row.get(c) ?? 0)) / n)}%`.padStart(6))].join(""));
}
