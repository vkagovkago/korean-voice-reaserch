import { useState, type RefObject } from "react";
import { DIALECT_OPTIONS, useDialect } from "../dialect.ts";
import { useT } from "../i18n.ts";
import { Menu } from "./Controls.tsx";
import { Chip } from "./Screen.tsx";

/** A key: what it types (conjoining jamo, a mark), and what it shows. */
type Key = { type: string; label: string };

const conj = (from: number, labels: string) => [...labels].map((label, i) => ({ type: String.fromCodePoint(from + i), label }));
// an archaic or cluster jamo shown on its own: the fillers make it draw as the letter it is
const lone = (c: number, slot: "initial" | "vowel" | "final"): Key => ({
  type: String.fromCodePoint(c),
  label: slot === "initial" ? String.fromCodePoint(c, 0x1160) : slot === "vowel" ? String.fromCodePoint(0x115f, c) : String.fromCodePoint(0x115f, 0x1160, c),
});

/** The Old Hangul keys, as the Korean front end reads them (crates/chimera/src/phonetics/korean.rs). */
const TABS: { id: string; label: string; keys: Key[] }[] = [
  {
    id: "initial",
    label: "Initial",
    keys: [
      ...conj(0x1100, "ㄱㄲㄴㄷㄸㄹㅁㅂㅃㅅㅆㅇㅈㅉㅊㅋㅌㅍㅎ"),
      { type: "ᅀ", label: "ㅿ" },
      { type: "ᅌ", label: "ㆁ" },
      { type: "ᅙ", label: "ㆆ" },
      { type: "ᄫ", label: "ㅸ" },
      lone(0x1147, "initial"),
      ...[0x111e, 0x1120, 0x1121, 0x1122, 0x1123, 0x1127, 0x1129, 0x112d, 0x112f, 0x1132, 0x1136].map((c) => lone(c, "initial")),
    ],
  },
  {
    id: "vowel",
    label: "Vowel",
    keys: [...conj(0x1161, "ㅏㅐㅑㅒㅓㅔㅕㅖㅗㅘㅙㅚㅛㅜㅝㅞㅟㅠㅡㅢㅣ"), { type: "ᆞ", label: "ㆍ" }, { type: "ᆡ", label: "ㆎ" }, lone(0x11a2, "vowel")],
  },
  {
    id: "final",
    label: "Final",
    keys: [
      ...conj(0x11a8, "ㄱㄲㄳㄴㄵㄶㄷㄹㄺㄻㄼㄽㄾㄿㅀㅁㅂㅄㅅㅆㅇㅈㅊㅋㅌㅍㅎ"),
      { type: "ᇫ", label: "ㅿ" },
      { type: "ᇰ", label: "ㆁ" },
      { type: "ᇹ", label: "ㆆ" },
      lone(0x11d9, "final"),
    ],
  },
  {
    id: "marks",
    label: "Marks",
    keys: [
      // tone dots (15th century) and the acute that marks a high syllable (Gyeongsang)
      { type: "〮", label: "〮" },
      { type: "〯", label: "〯" },
      { type: "́", label: "´" },
      { type: ".", label: "." },
      { type: ",", label: "," },
    ],
  },
];
const NEXT: Record<string, string> = { initial: "vowel", vowel: "final", final: "initial", marks: "marks" };

/**
 * How Hangul reads (standard, a regional dialect, the 15th century), and an on-screen keyboard for
 * what an ordinary one can't type: Old Hangul and Jeju letters, clusters, tone dots. It types into
 * `target` at its caret without focusing it (a phone's own keyboard stays down).
 */
export function KoreanInput(props: { target: RefObject<HTMLTextAreaElement | null>; value: string; onChange: (v: string) => void }) {
  const { dialect, setDialect } = useDialect();
  const t = useT();
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState("initial");

  const edit = (f: (before: string, after: string) => [string, string]) => {
    const ta = props.target.current;
    const v = props.value;
    const [s, e] = ta ? [ta.selectionStart, ta.selectionEnd] : [v.length, v.length];
    const [before, after] = f(v.slice(0, s), v.slice(e));
    props.onChange(before + after);
    // the caret after what was typed, once React has written the new text
    requestAnimationFrame(() => props.target.current?.setSelectionRange(before.length, before.length));
  };
  const type = (k: Key) => {
    edit((b, a) => [b + k.type, a]);
    setTab(NEXT[tab]);
  };
  const del = () => edit((b, a) => [[...b].slice(0, -1).join(""), a]);

  const keys = TABS.find((x) => x.id === tab)!.keys;
  return (
    <div className="korean-input">
      <div className="korean-row">
        <Menu label="Korean reading" value={dialect} options={DIALECT_OPTIONS.map((o) => ({ value: o.value, label: t(o.label) }))} onChange={setDialect} />
        <Chip on={open} onClick={() => setOpen(!open)} title="Type Old Hangul and Jeju letters, clusters and tone dots">
          Old Hangul keyboard
        </Chip>
      </div>
      {open && (
        <div className="jamo-kbd" role="group" aria-label={t("Old Hangul keyboard")}>
          <div className="jamo-tabs" role="tablist">
            {TABS.map((x) => (
              <Chip key={x.id} on={tab === x.id} role="tab" aria-selected={tab === x.id} onClick={() => setTab(x.id)}>
                {x.label}
              </Chip>
            ))}
          </div>
          <div className="jamo-keys">
            {keys.map((k) => (
              <button key={k.type} type="button" className="jamo-key" onClick={() => type(k)} aria-label={k.label}>
                {k.label}
              </button>
            ))}
            <button type="button" className="jamo-key wide" onClick={() => edit((b, a) => [b + " ", a])}>
              {t("Space")}
            </button>
            <button type="button" className="jamo-key wide" onClick={del} aria-label={t("Delete")}>
              ⌫
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
