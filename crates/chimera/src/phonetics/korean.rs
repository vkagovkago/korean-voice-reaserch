//! Hangul lyrics → sung syllables. Each Hangul block is one sung syllable, so a block maps to one
//! note. The block keeps its spelling as the note's text; its sounds follow standard Seoul
//! pronunciation inside a word: a coda moves onto a following silent ㅇ (먹어 → 머거), codas
//! neutralise to seven sounds, obstruents nasalise before nasals (국물 → 궁물), ㄴ/ㄹ meet as [ll],
//! ㅎ aspirates its neighbour (좋다 → 조타) or drops before a vowel and lets the consonant before it
//! move over (싫어 → 시러), ㄷ ㅌ palatalise before 이 (같이 → 가치), and obstruents tense after an
//! obstruent coda.
//!
//! Stops say their own aspiration (Segment::aspirated): aspirated ㅋ ㅌ ㅍ ㅊ are aspirated, and
//! tense ㄲ ㄸ ㅃ ㅉ are voiceless and unaspirated, and they and ㅆ stiffen the folds into the vowel
//! (Segment::fortis: a pressed voice where a lax stop's vowel starts breathy).
//! Lax stops aspirate briefly word-initially and aspirated ones long (Segment::vot); lax stops are
//! voiced between voiced sounds.
//! ㅈ ㅊ ㅉ are alveolo-palatal [tɕ], and ㅅ ㅆ before [i] or [j] are [ɕ].

use super::syllables::FrontEnd;
use super::{Segment, Syllable, Tone};

const BASE: u32 = 0xAC00;

/// A Hangul syllable block, or a conjoining jamo (how Old Hangul is written: ᄒᆞᆫ is ᄒ ᆞ ᆫ), or a
/// tone dot (방점 〮 〯, read and dropped).
pub fn is_hangul(c: char) -> bool {
    matches!(c, '\u{AC00}'..='\u{D7A3}' | '\u{1100}'..='\u{11FF}' | '\u{A960}'..='\u{A97F}' | '\u{D7B0}'..='\u{D7FF}' | '\u{302E}' | '\u{302F}')
}

/// Inside a Korean word: Hangul, and the acute accent that marks a high syllable (가́지) for a
/// pitch-accent dialect.
pub fn in_korean_word(c: char) -> bool {
    is_hangul(c) || c == '\u{301}'
}

/// A regional profile: which sound features of a dialect apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dialect {
    /// standard Seoul speech: no lexical pitch, accent marks ignored
    #[default]
    Seoul,
    Central,
    /// Gyeongsang (a pilot profile, three features most accounts agree on): ㅓ and ㅡ merge as [ə];
    /// ㅆ is said as ㅅ (쌀 = 살); pitch accent, a syllable marked with an acute (가́지) said high.
    /// Which syllables are high is the word's, by region and speaker; with no lexicon here, the
    /// input marks them. Not modelled: rising (long) tones, regional and generational differences.
    Gyeongsang,
    Southwest,
    Northwest,
    Northeast,
    /// Jeju: mid back rounded [ɔ] for ㆍ and [jɔ] for ᆢ (Kim 2005).
    Jeju,
    /// the 15th century, as most reconstructions have it (이기문's vowel system): ㅓ [ə], ㅡ [ɨ],
    /// ㆍ by its hypothesis; ㅐ ㅔ ㅚ ㅟ ㅢ ㆎ diphthongs ending in [j] (ㅐ [aj]); a final ㅅ said [s]
    /// (eight finals, not seven); and the tone dots: one (去聲) high, two (上聲) rising.
    Middle,
}

impl Dialect {
    pub fn from_name(name: &str) -> Option<Dialect> {
        match name {
            "seoul" => Some(Dialect::Seoul),
            "central" => Some(Dialect::Central),
            "gyeongsang" | "southeast" => Some(Dialect::Gyeongsang),
            "southwest" => Some(Dialect::Southwest),
            "northwest" => Some(Dialect::Northwest),
            "northeast" => Some(Dialect::Northeast),
            "jeju" => Some(Dialect::Jeju),
            "middle" => Some(Dialect::Middle),
            _ => None,
        }
    }
}

/// How the Korean front end reads: ㆍ's hypothesis and the dialect.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub arae_a: &'static str,
    pub dialect: Dialect,
    /// Old Hangul's ㅅ-series clusters (ᄭ ᄯ ᄲ ᄶ) read as tense consonants (ㄲ ㄸ ㅃ ㅉ) instead of
    /// as clusters ([sk]): the two readings argued for; sources not recorded here yet
    pub siot_tense: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { arae_a: ARAE_A[0].ipa, dialect: Dialect::Seoul, siot_tense: false }
    }
}

/// A reading's vowel for a vowel letter: (glide, vowel, an offglide [j] after it), or None for the
/// modern one.
fn reading_vowel(letter: char, o: &Options) -> Option<(&'static str, &'static str, bool)> {
    use Dialect::*;
    Some(match (o.dialect, letter) {
        (Middle, 'ㅓ') => ("", "ə", false),
        (Middle, 'ㅕ') => ("j", "ə", false),
        (Middle, 'ㅝ') => ("w", "ə", false),
        (Middle, 'ㅡ') => ("", "ɨ", false),
        (Middle, 'ㅐ') => ("", "a", true),
        (Middle, 'ㅒ') => ("j", "a", true),
        (Middle, 'ㅙ') => ("w", "a", true),
        (Middle, 'ㅔ') => ("", "ə", true),
        (Middle, 'ㅖ') => ("j", "ə", true),
        (Middle, 'ㅞ') => ("w", "ə", true),
        (Middle, 'ㅚ') => ("", "o", true),
        (Middle, 'ㅟ') => ("", "u", true),
        (Middle, 'ㅢ') => ("", "ɨ", true),
        (Jeju, 'ㆍ') => ("", "ɔ", false),
        (Jeju, 'ᆢ') => ("j", "ɔ", false),
        (Central, 'ㅐ') => ("", "e", false),
        (Central, 'ㅒ') => ("j", "e", false),
        (Central, 'ㅙ') => ("w", "e", false),
        (Southwest, 'ㅢ') => ("", "ɯ", false),
        (Northeast, 'ㅢ') => ("", "i", false),
        (Gyeongsang, 'ㅓ' | 'ㅡ') => ("", "ə", false),
        (Gyeongsang, 'ㅕ') => ("j", "ə", false),
        (Gyeongsang, 'ㅝ') => ("w", "ə", false),
        // ㆎ is ㆍ and [j] in any reading
        (_, 'ㆎ') => ("", o.arae_a, true),
        _ => return None,
    })
}

/// A sound-value hypothesis for an Old Hangul letter whose 15th-century value is argued over.
pub struct Hypothesis {
    pub ipa: &'static str,
    pub label: &'static str,
    /// where it is argued; None: not recorded here yet
    pub source: Option<&'static str>,
}

/// ㆍ (arae-a), the one Old Hangul letter read here whose value is in real dispute; the first is
/// the default. A render of each is a demonstration of the hypothesis, not evidence for it. Under
/// the first, ㆍ sounds as this front end's modern ㅓ does.
pub const ARAE_A: [Hypothesis; 3] = [
    Hypothesis { ipa: "ʌ", label: "back unrounded, low-mid", source: Some("이기문 (1972), 국어음운사연구") },
    Hypothesis { ipa: "ɐ", label: "central, near-low", source: None },
    Hypothesis { ipa: "ɔ", label: "back rounded, low-mid", source: None },
];

/// Old Hangul's other letters, with the values most accounts agree on: ㅿ [z], ㆁ [ŋ] (as an
/// initial too), ㆆ [ʔ], ㅸ [β]. As letters (compatibility jamo) the rules below handle them as they
/// do the modern ones. Not read: consonant clusters (ㅺ ㅄ …), ㆎ ᆢ, other archaic jamo (dropped),
/// and the tone dots. Everything else keeps its modern value and the modern rules.
fn old_initial(c: char) -> Option<char> {
    Some(match c {
        '\u{1140}' => 'ㅿ',
        '\u{114C}' => 'ㆁ',
        '\u{1159}' => 'ㆆ',
        '\u{112B}' => 'ㅸ',
        _ => return None,
    })
}

fn old_final(c: char) -> Option<Vec<char>> {
    Some(match c {
        '\u{11EB}' => vec!['ㅿ'],
        '\u{11F0}' => vec!['ㅇ'], // ㆁ as a final is the modern final ㅇ
        '\u{11F9}' => vec!['ㆆ'],
        // ㅭ (ᇙ, ㄹ then ㆆ): said ㄹ; its ㆆ tenses the consonant after it in the word (호ᇙ 배)
        '\u{11D9}' => vec!['ㄹ', 'ㆆ'],
        _ => return None,
    })
}

/// A word's blocks: precomposed syllables, and runs of conjoining jamo (initials, then medials,
/// then finals), with ㆍ read as `arae_a`.
fn blocks_of(word: &str, arae_a: &'static str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    // what the last jamo was: 0 none (a new block starts), 1 initial, 2 medial, 3 final
    let mut stage = 0;
    for c in word.chars() {
        let u = c as u32;
        if ('\u{AC00}'..='\u{D7A3}').contains(&c) {
            out.push(decompose(c));
            stage = 2; // a final jamo may follow a precomposed block
            continue;
        }
        let cluster = CLUSTERS.iter().find(|(j, _)| *j == c).map(|(_, cs)| *cs);
        let initial = match u {
            0x1100..=0x1112 => Some(Some(INITIALS[(u - 0x1100) as usize]).filter(|&o| o != 'ㅇ')),
            // ᅇ (double ㅇ, ᄒᆡᅇᅧ): its value is argued over; read as a silent initial
            0x1147 => Some(None),
            _ => old_initial(c).map(Some).or(cluster.map(|cs| cs.last().copied())),
        };
        // (glide, vowel, letter); ㆎ is ㆍ with an offglide, read as such when a reading has one
        let medial = match u {
            0x1161..=0x1175 => Some((MEDIALS[(u - 0x1161) as usize], MEDIAL_LETTERS[(u - 0x1161) as usize])),
            0x119E => Some((("", arae_a), 'ㆍ')),
            0x11A1 => Some((("", arae_a), 'ㆎ')),
            0x11A2 => Some((("j", arae_a), 'ᆢ')),
            _ => None,
        };
        let fin: Option<Vec<char>> = match u {
            0x11A8..=0x11C2 => Some(FINALS[(u - 0x11A8 + 1) as usize].chars().collect()),
            _ => old_final(c),
        };
        if let Some(o) = initial {
            if stage != 1 {
                out.push(Block { text: String::new(), onset: None, glide: "", vowel: "", coda: vec![], high: false, medial: ' ', pre: vec![], tone: 0 });
            }
            let b = out.last_mut().unwrap();
            b.onset = o;
            b.pre = cluster.map(|cs| cs[..cs.len() - 1].to_vec()).unwrap_or_default();
            b.text.push(c);
            stage = 1;
        } else if let Some(((glide, vowel), letter)) = medial {
            if stage != 1 {
                out.push(Block { text: String::new(), onset: None, glide: "", vowel: "", coda: vec![], high: false, medial: ' ', pre: vec![], tone: 0 });
            }
            let b = out.last_mut().unwrap();
            (b.glide, b.vowel, b.medial) = (glide, vowel, letter);
            b.text.push(c);
            stage = 2;
        } else if let (Some(f), true) = (fin, stage >= 2) {
            let b = out.last_mut().unwrap();
            b.coda.extend(f);
            b.text.push(c);
            stage = 3;
        } else if let Some(b) = out.last_mut().filter(|_| stage > 0) {
            // an accent mark, a tone dot or a jamo not read: kept in the text; the accent marks
            // the block high
            b.high |= c == '\u{301}';
            b.tone = match c {
                '\u{302E}' => 1,
                '\u{302F}' => 2,
                _ => b.tone,
            };
            b.text.push(c);
        }
    }
    // a block with no vowel (a stray initial) has nothing to sing
    out.retain(|b| !b.vowel.is_empty());
    out
}

/// Initial consonants in Unicode order. 'ㅇ' as an initial is silent.
const INITIALS: [char; 19] = ['ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ'];

/// Medial vowels in Unicode order as (glide, vowel).
const MEDIALS: [(&str, &str); 21] = [
    ("", "a"), ("", "ɛ"), ("j", "a"), ("j", "ɛ"), ("", "ʌ"), ("", "e"), ("j", "ʌ"), ("j", "e"), // ㅏㅐㅑㅒㅓㅔㅕㅖ
    ("", "o"), ("w", "a"), ("w", "ɛ"), ("w", "e"), ("j", "o"), // ㅗㅘㅙㅚㅛ
    ("", "u"), ("w", "ʌ"), ("w", "e"), ("w", "i"), ("j", "u"), // ㅜㅝㅞㅟㅠ
    ("", "ɯ"), ("ɰ", "i"), ("", "i"), // ㅡㅢㅣ
];

/// Final consonants in Unicode order, as one or two letters ("" is no coda).
const FINALS: [&str; 28] = [
    "", "ㄱ", "ㄲ", "ㄱㅅ", "ㄴ", "ㄴㅈ", "ㄴㅎ", "ㄷ", "ㄹ", "ㄹㄱ", "ㄹㅁ", "ㄹㅂ", "ㄹㅅ", "ㄹㅌ", "ㄹㅍ", "ㄹㅎ",
    "ㅁ", "ㅂ", "ㅂㅅ", "ㅅ", "ㅆ", "ㅇ", "ㅈ", "ㅊ", "ㅋ", "ㅌ", "ㅍ", "ㅎ",
];

/// One block with its letters, rewritten in place by the sound rules.
struct Block {
    text: String,
    /// None: silent ㅇ
    onset: Option<char>,
    glide: &'static str,
    vowel: &'static str,
    coda: Vec<char>,
    /// marked high (an acute after it)
    high: bool,
    /// the vowel letter (compatibility jamo: ㅐ, ㆍ, …), which a historical or regional reading reads
    medial: char,
    /// consonants before the onset in an Old Hangul cluster (ᄢ: ㅂ ㅅ before ㄱ)
    pre: Vec<char>,
    /// the tone dots after it: 0 none (平聲, low), 1 one (去聲, high), 2 two (上聲, rising)
    tone: u8,
}

/// The vowel letters in Unicode order, as `MEDIALS`.
const MEDIAL_LETTERS: [char; 21] = ['ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ'];

/// Old Hangul's initial clusters (합용병서), as the consonants written: the ㅂ series read as
/// clusters; the ㅅ series as clusters or as tense consonants (`Options::siot_tense`).
const CLUSTERS: [(char, &[char]); 11] = [
    ('\u{111E}', &['ㅂ', 'ㄱ']), ('\u{1120}', &['ㅂ', 'ㄷ']), ('\u{1121}', &['ㅂ', 'ㅅ']), ('\u{1122}', &['ㅂ', 'ㅅ', 'ㄱ']),
    ('\u{1123}', &['ㅂ', 'ㅅ', 'ㄷ']), ('\u{1127}', &['ㅂ', 'ㅈ']), ('\u{1129}', &['ㅂ', 'ㅌ']), ('\u{112D}', &['ㅅ', 'ㄱ']),
    ('\u{112F}', &['ㅅ', 'ㄷ']), ('\u{1132}', &['ㅅ', 'ㅂ']), ('\u{1136}', &['ㅅ', 'ㅈ']),
];

fn decompose(c: char) -> Block {
    let i = c as u32 - BASE;
    let (cho, jung, jong) = ((i / 588) as usize, ((i % 588) / 28) as usize, (i % 28) as usize);
    let onset = Some(INITIALS[cho]).filter(|&o| o != 'ㅇ');
    let (glide, vowel) = MEDIALS[jung];
    Block { text: c.to_string(), onset, glide, vowel, coda: FINALS[jong].chars().collect(), high: false, medial: MEDIAL_LETTERS[jung], pre: vec![], tone: 0 }
}

/// A coda's sound once it can't move on: one of ㄱ ㄴ ㄷ ㄹ ㅁ ㅂ ㅇ.
fn neutral(c: char) -> char {
    match c {
        'ㄱ' | 'ㄲ' | 'ㅋ' => 'ㄱ',
        'ㄷ' | 'ㅅ' | 'ㅆ' | 'ㅈ' | 'ㅊ' | 'ㅌ' | 'ㅎ' | 'ㅿ' => 'ㄷ',
        'ㅂ' | 'ㅍ' => 'ㅂ',
        other => other,
    }
}

/// [`neutral`], keeping a final ㅅ as [s] when `keep_s` (the 15th century's eight finals).
fn neutral_in(c: char, keep_s: bool) -> char {
    if keep_s && c == 'ㅅ' { c } else { neutral(c) }
}

/// Which letter of a two-letter coda sounds when nothing follows to take the other.
fn simplify(coda: &[char]) -> Option<char> {
    match coda {
        [] => None,
        [c] => Some(*c),
        ['ㄹ', b @ ('ㄱ' | 'ㅁ' | 'ㅍ')] => Some(*b),
        [a, _] => Some(*a),
        _ => unreachable!("codas have at most two letters"),
    }
}

fn aspirated(c: char) -> Option<char> {
    Some(match c {
        'ㄱ' => 'ㅋ',
        'ㄷ' => 'ㅌ',
        'ㅂ' => 'ㅍ',
        'ㅈ' => 'ㅊ',
        _ => return None,
    })
}

fn tense(c: char) -> char {
    match c {
        'ㄱ' => 'ㄲ',
        'ㄷ' => 'ㄸ',
        'ㅂ' => 'ㅃ',
        'ㅅ' => 'ㅆ',
        'ㅈ' => 'ㅉ',
        other => other,
    }
}

/// Apply the sound rules at the boundary between block `a` and the next block `b`.
fn join(a: &mut Block, b: &mut Block, dialect: Dialect) {
    let keep_s = dialect == Dialect::Middle;
    // a coda before a silent ㅇ moves over (ㅎ drops, ㅇ stays: 좋아 → 조아, 빵야 → 빵야); after a
    // dropped ㅎ the consonant before it moves (싫어 → 시러, 많아 → 마나), and ㄷ ㅌ moving onto
    // 이 palatalise (굳이 → 구지, 같이 → 가치)
    if b.onset.is_none() {
        if a.coda.last() == Some(&'ㅎ') {
            a.coda.pop();
        }
        if a.coda.last().is_some_and(|&c| c != 'ㅇ') {
            let c = a.coda.pop().unwrap();
            let before_i = b.vowel == "i" && b.glide.is_empty() && dialect != Dialect::Northwest;
            b.onset = Some(match c {
                'ㄷ' if before_i => 'ㅈ',
                'ㅌ' if before_i => 'ㅊ',
                _ => c,
            });
        }
        return;
    }
    // ㅭ's ㆆ (15th-century spelling) tenses the consonant after it and is not said itself
    if a.coda.len() == 2 && a.coda[1] == 'ㆆ' {
        a.coda.pop();
        b.onset = Some(tense(b.onset.unwrap()));
    }
    // ㅎ meets a stop or affricate: one aspirated consonant (좋다 → 조타, 축하 → 추카)
    if a.coda.last() == Some(&'ㅎ') {
        a.coda.pop();
        let o = b.onset.unwrap();
        b.onset = Some(match o {
            'ㅅ' => 'ㅆ',
            // ㄶ and ㅎ read [nn] before ㄴ (않는 → 안는, 놓는 → 논는); ㅀ reads [ll] (싫니 → 실리)
            'ㄴ' if a.coda == ['ㄹ'] => 'ㄹ',
            'ㄴ' => {
                a.coda = vec!['ㄴ'];
                'ㄴ'
            }
            _ => aspirated(o).unwrap_or(o),
        });
        if a.coda.is_empty() {
            return;
        }
    }
    let Some(c) = simplify(&a.coda).map(|c| neutral_in(c, keep_s)) else { return };
    a.coda = vec![c];
    let o = b.onset.unwrap();
    if o == 'ㅎ' && matches!(c, 'ㄱ' | 'ㄷ' | 'ㅂ') {
        a.coda.clear();
        b.onset = aspirated(c);
        return;
    }
    // ㄹ after any coda but ㄴ/ㄹ reads ㄴ (국립 → 궁닙); ㄴ and ㄹ together read [ll] (신라 → 실라)
    let o = match (c, o) {
        ('ㄹ', 'ㄴ') | ('ㄴ', 'ㄹ') => {
            a.coda = vec!['ㄹ'];
            'ㄹ'
        }
        (_, 'ㄹ') if c != 'ㄹ' => 'ㄴ',
        _ => o,
    };
    b.onset = Some(o);
    // obstruent codas nasalise before nasals, and tense the obstruent after them
    if matches!(c, 'ㄱ' | 'ㄷ' | 'ㅂ') {
        if matches!(o, 'ㄴ' | 'ㅁ') {
            a.coda = vec![match c {
                'ㄱ' => 'ㅇ',
                'ㄷ' => 'ㄴ',
                _ => 'ㅁ',
            }];
        } else {
            b.onset = Some(tense(o));
        }
    }
}

/// An initial's sound, and its aspiration when the letter decides it (None: the planner's default).
/// Korean's three stops, word-initially all voiceless, told apart by how long they aspirate:
/// aspirated ㅋ ㅌ ㅍ ㅊ long, lax ㄱ ㄷ ㅂ ㅈ briefly, tense ㄲ ㄸ ㅃ ㅉ not at all (word-initial voice
/// onset times of about 70, 57 and 24 ms in the render; modern Seoul speech measures aspirated 57-75 ms, lax 50-65, tense 16-24, the 2026 Korean phonetics notes). Between voiced sounds a lax stop is voiced.
/// written 13 ms under the target: the render's voice onset lands about that much after the
/// release than the value asks for (`chimera diagnose korean`: 45 → 57 ms lax, 70 → 83 aspirated)
const ASPIRATED_VOT: f64 = 0.057;
const LAX_VOT: f64 = 0.045;
/// a tense stop's release gap, written 5 ms under the planner's default unaspirated one: the render
/// then lands at 20-23 ms, the middle of Seoul's 16-24 (`chimera diagnose korean`; 24-27 before)
const TENSE_VOT: f64 = 0.007;

/// An initial → its IPA, whether it aspirates, and for how long (s).
fn onset_ipa(o: char, voiced_before: bool, front: bool, after_l: bool) -> (&'static str, Option<bool>, Option<f64>) {
    let lax = |voiced: &'static str, voiceless: &'static str| {
        if voiced_before {
            (voiced, None, None)
        } else {
            (voiceless, Some(true), Some(LAX_VOT))
        }
    };
    let aspirated = |ipa: &'static str| (ipa, Some(true), Some(ASPIRATED_VOT));
    let tense = |ipa: &'static str| (ipa, Some(false), Some(TENSE_VOT));
    match o {
        'ㄱ' => lax("g", "k"),
        'ㅋ' => aspirated("k"),
        'ㄲ' => tense("k"),
        'ㄷ' => lax("d", "t"),
        'ㅌ' => aspirated("t"),
        'ㄸ' => tense("t"),
        'ㅂ' => lax("b", "p"),
        'ㅍ' => aspirated("p"),
        'ㅃ' => tense("p"),
        'ㅈ' => lax("dʑ", "tɕ"),
        'ㅊ' => aspirated("tɕ"),
        'ㅉ' => tense("tɕ"),
        'ㅅ' | 'ㅆ' if front => ("ɕ", None, None),
        'ㅅ' | 'ㅆ' => ("s", None, None),
        'ㅎ' if voiced_before => ("ɦ", None, None),
        'ㅎ' => ("h", None, None),
        'ㄴ' => ("n", None, None),
        'ㅁ' => ("m", None, None),
        'ㄹ' if after_l => ("l", None, None),
        'ㄹ' => ("ɾ", None, None),
        'ㅿ' => ("z", None, None),
        'ㆁ' => ("ŋ", None, None),
        'ㆆ' => ("ʔ", None, None),
        'ㅸ' => ("β", None, None),
        _ => unreachable!("not an initial: {o}"),
    }
}

fn coda_ipa(c: char) -> &'static str {
    match c {
        'ㄱ' => "k",
        'ㄴ' => "n",
        'ㄷ' => "t",
        'ㄹ' => "l",
        'ㅁ' => "m",
        'ㅂ' => "p",
        'ㅇ' => "ŋ",
        'ㆆ' => "ʔ",
        'ㅅ' => "s",
        _ => unreachable!("not a neutral coda: {c}"),
    }
}

/// One word of Hangul (a run of blocks) → its sung syllables, the last marked as the word's end.
pub fn word_syllables(word: &str) -> Vec<Syllable> {
    word_syllables_with(word, &Options::default())
}

/// [`word_syllables`] read with `o`: ㆍ as `o.arae_a` (one of [`ARAE_A`]'s values, or any vowel),
/// in `o.dialect`.
pub fn word_syllables_with(word: &str, o: &Options) -> Vec<Syllable> {
    // a word the rules can't read (they need its parts) is read from its respelling, and each
    // syllable keeps the letter the lyrics wrote
    let Some((_, said)) = SPELLED_AS.iter().find(|(w, _)| *w == word) else { return syllables_of(word, o) };
    let mut out = syllables_of(said, o);
    for (y, c) in out.iter_mut().zip(word.chars()) {
        y.text = c.to_string();
    }
    out
}

/// Words whose sounds depend on their parts, which a spelling alone doesn't give: compounds that
/// insert ㄴ before 이 야 여 요 유 (솜이불 [솜니불]), and Sino-Korean words that tense after ㄹ or
/// ㄴ (갈등 [갈뜽]); source: the 2026 phonological-process notes. Each respelling keeps the block count.
const SPELLED_AS: &[(&str, &str)] = &[
    ("솜이불", "솜니불"), ("홑이불", "혼니불"), ("한여름", "한녀름"), ("늦여름", "는녀름"), ("색연필", "생년필"),
    ("물약", "물략"), ("식용유", "시굥뉴"), ("눈요기", "눈뇨기"), ("꽃잎", "꼰닙"), ("깻잎", "깬닙"),
    ("나뭇잎", "나문닙"), ("담요", "담뇨"), ("맨입", "맨닙"), ("서울역", "서울력"), ("휘발유", "휘발류"),
    ("막일", "망닐"), ("갈등", "갈뜽"), ("발달", "발딸"), ("발전", "발쩐"), ("결정", "결쩡"), ("문법", "문뻡"),
    ("사건", "사껀"), ("인기", "인끼"), ("물건", "물껀"), ("효과", "효꽈"), ("증거", "증꺼"), ("성격", "성껵"),
    ("교과서", "교꽈서"), ("길가", "길까"), ("강가", "강까"), ("손등", "손뜽"), ("밤길", "밤낄"),
];

fn syllables_of(word: &str, opts: &Options) -> Vec<Syllable> {
    let gyeongsang = opts.dialect == Dialect::Gyeongsang;
    let middle = opts.dialect == Dialect::Middle;
    let northeast = opts.dialect == Dialect::Northeast;
    let mut blocks = blocks_of(word, opts.arae_a);
    // the reading's vowels (Gyeongsang's ㅆ as ㅅ is read with the onset, below), and the ㅅ-series
    // clusters as tense consonants when that is the reading asked for
    let mut offglide = vec![false; blocks.len()];
    for (b, off) in blocks.iter_mut().zip(offglide.iter_mut()) {
        if let Some((g, v, o)) = reading_vowel(b.medial, opts) {
            (b.glide, b.vowel, *off) = (g, v, o);
        }
        if opts.siot_tense
            && b.pre.last() == Some(&'ㅅ')
            && let Some(o) = b.onset.filter(|o| matches!(o, 'ㄱ' | 'ㄷ' | 'ㅂ' | 'ㅈ'))
        {
            b.pre.pop();
            b.onset = Some(tense(o));
        }
    }
    for i in 1..blocks.len() {
        let (l, r) = blocks.split_at_mut(i);
        join(l.last_mut().unwrap(), &mut r[0], opts.dialect);
    }
    if let Some(last) = blocks.last_mut() {
        last.coda = simplify(&last.coda).map(|c| neutral_in(c, middle)).into_iter().collect();
    }
    let mut out = Vec::with_capacity(blocks.len());
    for (i, b) in blocks.iter().enumerate() {
        let prev_coda = if i > 0 { blocks[i - 1].coda.last().copied() } else { None };
        let voiced_before = i > 0 && prev_coda.is_none_or(|c| matches!(c, 'ㄴ' | 'ㄹ' | 'ㅁ' | 'ㅇ'));
        let front = b.vowel == "i" && b.glide != "w" || b.glide == "j";
        // an Old Hangul cluster's first consonants (ᄢ: [p s] before ㄱ), plain and unaspirated
        let clustered = !b.pre.is_empty();
        let pre = b.pre.iter().map(|&c| Segment { aspirated: (c == 'ㅂ').then_some(false), ..Segment::new(if c == 'ㅂ' { "p" } else { "s" }) });
        let mut onset: Vec<Segment> = pre
            .chain(b.onset.map(|o| {
                let o = if gyeongsang && o == 'ㅆ' { 'ㅅ' } else { o };
                let (ipa, aspirated, vot) = onset_ipa(o, voiced_before && !clustered, front, prev_coda == Some('ㄹ'));
                // Northern Hamgyong and the northwestern profile use alveolar affricates.
                let ipa = if northeast || opts.dialect == Dialect::Northwest {
                    match ipa {
                        "tɕ" => "ts",
                        "dʑ" => "dz",
                        other => other,
                    }
                } else {
                    ipa
                };
                // after a cluster's consonants a stop doesn't aspirate (as after English /s/)
                let (aspirated, vot) = if clustered { (aspirated.map(|_| false), None) } else { (aspirated, vot) };
                // closure lengths (VCV, 2026 Korean phonetics notes): tense 123 ms, aspirated 84, lax 48;
                // a voiced [ɦ] between vowels is mostly dropped in real speech (90%), so it is brief
                let dur = match (aspirated, vot) {
                    (Some(false), _) => Some(0.12),
                    (Some(true), Some(v)) if v == ASPIRATED_VOT => Some(0.085),
                    (Some(true), _) => Some(0.05),
                    _ if ipa == "ɦ" => Some(0.045),
                    _ => None,
                };
                // ㄲ ㄸ ㅃ ㅉ ㅆ stiffen the folds into the vowel (fortis)
                let fortis = matches!(o, 'ㄲ' | 'ㄸ' | 'ㅃ' | 'ㅉ' | 'ㅆ').then_some(true);
                // a voiceless lax stop starts its vowel breathy (Cho, Jun & Ladefoged 2002)
                let breathy = (aspirated == Some(true) && vot == Some(LAX_VOT)).then_some(true);
                Segment { aspirated, vot, dur: if clustered { None } else { dur }, fortis, breathy, ..Segment::new(ipa) }
            }))
            .collect();
        // ㅢ after a consonant is plain [i] (희 → 히)
        if !b.glide.is_empty() && !(b.glide == "ɰ" && b.onset.is_some()) {
            onset.push(Segment::new(b.glide));
        }
        out.push(Syllable {
            text: b.text.clone(),
            onset,
            nucleus: Segment {
                tone: match (middle, northeast, b.tone, b.high) {
                    (true, _, 0, _) => Some(Tone::Low),
                    (true, _, 1, _) | (_, true, 1, _) | (_, true, 0, true) => Some(Tone::High),
                    (true, _, 2, _) | (_, true, 2, _) => Some(Tone::Rising),
                    _ => None,
                },
                stress: (gyeongsang && b.high).then_some(1),
                dur: ((middle || northeast) && b.tone == 2).then_some(0.25),
                ..Segment::new(b.vowel)
            },
            coda: offglide[i].then(|| Segment::new("j")).into_iter().chain(b.coda.iter().map(|&c| Segment::new(coda_ipa(c)))).collect(),
            melisma: None,
            word_end: i + 1 == blocks.len(),
            phrase_end: false,
            ..Default::default()
        });
    }
    out
}

/// The Korean front end: Hangul blocks, by the rules above (no data needed), read with its options.
#[derive(Default)]
pub struct Korean(pub Options);

impl FrontEnd for Korean {
    fn starts(&self, c: char) -> bool {
        is_hangul(c)
    }
    fn continues(&self, c: char) -> bool {
        in_korean_word(c)
    }
    fn word(&self, word: &str) -> Vec<Syllable> {
        word_syllables_with(word, &self.0)
    }
}
