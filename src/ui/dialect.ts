import { create } from "zustand";
import type { KoreanDialect } from "../wasm/engine.ts";

const KEY = "chimera.dialect";
const DIALECTS: KoreanDialect[] = ["seoul", "gyeongsang", "jeju", "middle"];

/** How Hangul reads, on the Speak and Sing screens alike (kept across visits). */
export const useDialect = create<{ dialect: KoreanDialect; setDialect: (d: KoreanDialect) => void }>((set) => {
  let dialect: KoreanDialect = "seoul";
  try {
    const v = localStorage.getItem(KEY) as KoreanDialect | null;
    if (v && DIALECTS.includes(v)) dialect = v;
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

/** The reading menu's choices (standard, the regional dialects, Middle Korean), as labels to translate. */
export const DIALECT_OPTIONS: { value: KoreanDialect; label: string }[] = [
  { value: "seoul", label: "Standard (Seoul)" },
  { value: "gyeongsang", label: "Gyeongsang dialect" },
  { value: "jeju", label: "Jeju dialect" },
  { value: "middle", label: "Middle Korean (15th c.)" },
];
