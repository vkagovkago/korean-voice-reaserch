import { useSyncExternalStore } from "react";

/** Interface language. English is the source text; Korean replaces a label only when KO has it. */
export type Lang = "en" | "ko";
const KEY = "chimera.lang";
const read = (): Lang => {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "en" || v === "ko") return v;
  } catch {}
  return "en";
};
let lang: Lang = read();
const subs = new Set<() => void>();

export function setLang(l: Lang) {
  lang = l;
  try {
    localStorage.setItem(KEY, l);
  } catch {}
  document.documentElement.lang = l;
  subs.forEach((f) => f());
}
/** Re-renders the caller when the language changes; returns the translator. */
export function useT(): (s: string) => string {
  useSyncExternalStore((f) => (subs.add(f), () => subs.delete(f)), () => lang);
  return tr;
}
export function useLang(): Lang {
  return useSyncExternalStore((f) => (subs.add(f), () => subs.delete(f)), () => lang);
}
export const tr = (s: string): string => (lang === "ko" ? (KO[s] ?? s) : s);

const KO: Record<string, string> = {
  "A voice grown from a simulated airway": "시뮬레이션한 기도(氣道)에서 자라난 목소리",
  Mode: "모드", Play: "연주", Speak: "말하기", Sing: "노래", Bestiary: "괴수도감", Beatbox: "비트박스",
  Volume: "볼륨", "Engine load": "엔진 부하",
  Dialect: "방언", Seoul: "서울", Gyeongsang: "경상", Jeju: "제주", "15th century": "15세기",
  "Mark a high syllable with an acute after it: 가́지": "높은 음절 뒤에 악센트 부호(́)를 붙이세요: 가́지",
  "Share of the audio thread's time budget used by the simulation": "시뮬레이션이 쓰는 오디오 스레드 시간 비율",
  "Source code on GitHub": "GitHub 소스 코드",
  "3D": "3D", "Sound pressure": "음압", Airflow: "공기 흐름", Grid: "격자", Body: "체형",
  Voice: "목소리", Breathe: "숨", Breath: "숨", breath: "숨",
  Tube: "관", "Vocal folds": "성대", Spectrum: "스펙트럼", Vowels: "모음", Keys: "건반",
  Spectrogram: "스펙트로그램", Airway: "기도", Controls: "조절", Creatures: "생물",
  "Gestural score": "조음 악보", Consonants: "자음", Examples: "예시", Lyrics: "가사",
  Song: "노래", Tune: "곡조", "Song voice": "노래 목소리", Pitch: "음높이", Vibrato: "비브라토",
  Tempo: "빠르기", Swing: "스윙", Bass: "베이스", Pads: "패드", Pattern: "패턴", Beat: "비트",
  "Play beat": "비트 재생", Transport: "재생 제어", "Step sequencer": "스텝 시퀀서",
  "Beat as text": "텍스트로 보는 비트", "Your beat": "내 비트", Calls: "울음소리",
  "What the keys play": "건반이 내는 소리", "What to say": "말할 내용",
  "Lyrics to sing": "부를 가사", "Type the words to sing": "부를 가사를 입력하세요",
  "Position in the song": "곡 안의 위치", Rate: "속도", Key: "조",
  Male: "남성", Female: "여성", Child: "아이",
  "Body between male, female and child": "남성, 여성, 아이 사이의 체형",
  Breathy: "숨 섞임", Pressed: "눌림", Fry: "발성 프라이", Rough: "거침", Bright: "밝음",
  Twang: "트왱", Ring: "링", Glide: "활음", Rounding: "원순", Protrude: "내밈",
  Jaw: "턱", Lips: "입술", "Lips open": "입술 벌림", Tongue: "혀", "Tongue body": "혀몸",
  "Tongue tip": "혀끝", "Tip curl": "혀끝 말림", "Soft palate": "연구개", Larynx: "후두",
  Pharynx: "인두", Trachea: "기관", Nostrils: "콧구멍", Mouth: "입", Root: "혀뿌리",
  "False folds": "가성대", Trunk: "코끼리 코", "Trunk valve": "코 밸브", Beak: "부리",
  "Left bronchus": "왼쪽 기관지", "Right bronchus": "오른쪽 기관지",
  "Left voice": "왼쪽 목소리", "Right voice": "오른쪽 목소리", "Held voice": "지속음",
  "Between two vowels": "두 모음 사이", "Vowel around it": "주변 모음",
  "F1 / F2": "F1 / F2", "Glottis to lips": "성문에서 입술까지", "Each tube unrolled": "펼쳐 놓은 각 관",
  "Slowed 100×": "100배 느리게", Sound: "소리", Whisper: "속삭임", Stop: "정지", "Starting…": "시작 중…", Labels: "이름표", Measurements: "치수",
  "Keep the breath on and shape the sound with the controls": "숨을 계속 내쉬며 조절 장치로 소리를 빚으세요",
  "Sing it again and record the face into a video": "다시 부르며 얼굴을 영상으로 녹화",
  "Vowel map. Drag to morph between vowels.": "모음 지도. 끌어서 모음 사이를 변형하세요.",
  "Waterfall spectrogram of the last two seconds of output": "최근 2초 출력의 워터폴 스펙트로그램",
  "Area function of the simulated tube, lit by sound pressure": "시뮬레이션 관의 단면적 함수 (음압으로 밝힘)",
  "Beat pattern: one sound per step": "비트 패턴: 한 스텝에 소리 하나",
};
