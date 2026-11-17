import { create } from "zustand";
import type { KoreanDialect } from "../wasm/engine.ts";

const KEY = "chimera.dialect";

/** The reading menu's choices (standard, six regional dialects and Middle Korean), as labels to translate. */
export const DIALECT_OPTIONS: { value: KoreanDialect; label: string }[] = [
  { value: "seoul", label: "Standard Korean" },
  { value: "central", label: "Central dialect (Seoul)" },
  { value: "southwest", label: "Southwestern dialect" },
  { value: "southeast", label: "Southeastern dialect" },
  { value: "jeju", label: "Jeju dialect" },
  { value: "northwest", label: "Northwestern dialect" },
  { value: "northeast", label: "Northeastern dialect" },
  { value: "middle", label: "Middle Korean (15th c.)" },
];

/** How Hangul reads, on the Speak and Sing screens alike (kept across visits). */
export const useDialect = create<{ dialect: KoreanDialect; setDialect: (d: KoreanDialect) => void }>((set) => {
  let dialect: KoreanDialect = "seoul";
  try {
    const v = localStorage.getItem(KEY) as KoreanDialect | null;
    const saved = v === "gyeongsang" ? "southeast" : v;
    if (saved && DIALECT_OPTIONS.some(({ value }) => value === saved)) dialect = saved;
  } catch {}
  return {
    dialect,
    setDialect: (d) => {
      try {
        localStorage.setItem(KEY, d);
      } catch {}
      set({ dialect: d });
    },
  };
});

/** Hangul, modern or Old (conjoining jamo): the text has Korean to read in a dialect. */
export const HANGUL = /[가-힣ᄀ-ᇿ]/;
