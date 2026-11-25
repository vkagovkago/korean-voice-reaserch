//! `chimera diagnose korean [outdir]`: Korean's three-way stop contrast, measured in the render.
//! Each place (ㄱ ㄷ ㅂ ㅈ) is said word-initially before /a/ as lax, aspirated and tense (가 카 까 …),
//! plus ㅅ/ㅆ, in the male and female voices, and measured against modern Seoul speech:
//!  - VOT: release to the folds' voicing onset (the glottis model's own voicing, so exact).
//!    Seoul norms: aspirated 57-75 ms, lax 50-65, tense 16-24 (the sources `korean.rs` cites)
//!  - onset F0: YIN 25 ms into the voice, in semitones over the lax stop at the same place.
//!    Norm: tense and aspirated about 5 semitones above lax (Beckman et al. 2014); checked as > 3
//!  - after ㅅ/ㅆ, the aspiration: the frication's end to the voicing onset
//!  - H1-A3 (dB), 30-50 ms into the voice: the spectral tilt, lowest after a tense stop (a pressed
//!    voice; Cho, Jun & Ladefoged 2002); lax should be the breathiest (highest)
//!
//! With an outdir it also writes each syllable's WAV, for a Korean speaker's ear.
//!
//! `chimera diagnose middle-korean [outdir]`: Old Hangul words said under each hypothesis for ㆍ
//! (`korean::ARAE_A`) in the 15th-century reading (where ㅓ is [ə], so [ʌ] is ㆍ's alone), the ㆍ
//! vowel's F1/F2 beside modern ㅏ ㅓ ㅗ, and WAVs to compare by ear. A
//! demonstration of each hypothesis in this voice; nothing here can say which is right.

use crate::{Ctx, FS, wav};
use chimera::analysis::fft::{Fft, hann, power_spectrum_db};
use chimera::analysis::lpc::{FormantOpts, estimate_formants};
use chimera::analysis::pitch::{YinOpts, yin_pitch};
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, HumanBody};
use chimera::math::log2;
use chimera::phonetics::korean::{ARAE_A, Dialect, Options, word_syllables, word_syllables_with};
use chimera::phonetics::Plan;
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::Segment;
use chimera::phonetics::sing::SUNG_VOT;
use chimera::voice::human_voice::{HumanVoice, SourcePatch};

/// (place, lax, aspirated, tense); ㅅ has no aspirated partner
const SETS: [(&str, &str, Option<&str>, &str); 5] =
    [("velar", "가", Some("카"), "까"), ("alveolar", "다", Some("타"), "따"), ("bilabial", "바", Some("파"), "빠"), ("affricate", "자", Some("차"), "짜"), ("fricative", "사", None, "싸")];
const VOICES: [(HumanBody, &str, f64); 2] = [(BODY_MALE, "male", 110.0), (BODY_FEMALE, "female", 200.0)];
const BLOCK: usize = 128;

pub struct Measured {
    /// release to voicing onset (ms)
    pub vot: f64,
    /// F0 25 ms into the voice (Hz; 0 if YIN found none)
    pub f0: f64,
    /// H1-A3 (dB), mean of the windows centred 30, 40 and 50 ms into the voice
    pub tilt: f64,
    pub out: Vec<f32>,
}

/// Say one Korean syllable word-initially and measure its onset.
pub fn measure(word: &str, body: HumanBody, name: &str, f0: f64) -> Measured {
    measure_syllables(word_syllables(word), body, name, f0)
}

/// [`measure`] with the voice onset times scaled as Sing scales them
fn measure_sung(word: &str, body: HumanBody, name: &str, f0: f64) -> Measured {
    let mut syls = word_syllables(word);
    for s in syls.iter_mut().flat_map(|s| s.onset.iter_mut()) {
        s.vot = s.vot.map(|v| v * SUNG_VOT);
    }
    measure_syllables(syls, body, name, f0)
}

fn measure_syllables(syls: Vec<chimera::phonetics::Syllable>, body: HumanBody, name: &str, f0: f64) -> Measured {
    let (plan, out, voicing) = say(syls, body, name, f0);
    let release = plan.segments[0].t1;
    let first = (release * FS / BLOCK as f64) as usize;
    let onset = (first..voicing.len()).find(|&i| voicing[i] > 0.5).unwrap_or(first) as f64 * BLOCK as f64 / FS;
    let frame = |t: f64, n: usize| -> Vec<f64> {
        let mid = (t * FS) as usize;
        out[mid - n / 2..mid + n / 2].iter().map(|x| *x as f64).collect()
    };
    let pitch = yin_pitch(&frame(onset + 0.025, 1024), FS, YinOpts { min_f0: Some(60.0), max_f0: Some(500.0), ..Default::default() }).f0;
    let tilt = [0.03, 0.04, 0.05].iter().map(|k| h1_a3(&frame(onset + k, 2048), if pitch > 0.0 { pitch } else { f0 })).sum::<f64>() / 3.0;
    Measured { vot: (onset - release) * 1000.0, f0: pitch, tilt, out }
}

/// Say syllables (each vowel 350 ms, stressed, with its tone): the plan, the audio and the voicing per block.
fn say(syls: Vec<chimera::phonetics::Syllable>, body: HumanBody, name: &str, f0: f64) -> (Plan, Vec<f32>, Vec<f64>) {
    let segs: Vec<Segment> = syls
        .iter()
        .flat_map(|s| s.onset.iter().cloned().chain([Segment { stress: Some(1), dur: Some(0.35), tone: s.nucleus.tone, ..Segment::new(&s.nucleus.ipa) }]).chain(s.coda.iter().cloned()))
        .collect();
    let plan = plan_utterance(&segs, &PlanOptions { body: name.into(), t0: 0.1, f0: Some(f0), ..Default::default() });
    let mut v = HumanVoice::new(FS, body, 2);
    v.set_source(&SourcePatch::f0(f0));
    v.schedule(&plan.gestures);
    let mut out = vec![0.0f32; (FS * (plan.end + 0.3)) as usize];
    let mut voicing = Vec::new();
    for c in out.chunks_mut(BLOCK) {
        v.render(c);
        voicing.push(v.glottis.voicing);
    }
    (plan, out, voicing)
}

/// F1 and F2 (Hz) of the first vowel, 30 ms at its middle
fn first_vowel_formants(plan: &Plan, out: &[f32]) -> (f64, f64) {
    let v = plan.segments.iter().find(|s| s.kind == "vowel").expect("a vowel");
    let (mid, w) = ((0.5 * (v.t0 + v.t1) * FS) as usize, (0.03 * FS) as usize);
    let frame: Vec<f64> = out[mid - w / 2..mid + w / 2].iter().map(|x| *x as f64).collect();
    let f = estimate_formants(&frame, FS, FormantOpts::default());
    (f[0].freq, f[1].freq)
}

/// Old Hangul words with ㆍ in their first syllable: 'one', 'heart' (with ㅿ), 'person', 'moon'
const MIDDLE_WORDS: [&str; 4] = ["ᄒᆞᆫ", "ᄆᆞᅀᆞᆷ", "ᄉᆞᄅᆞᆷ", "ᄃᆞᆯ"];

pub fn run_middle(ctx: &Ctx, a: &[String]) -> Result<(), String> {
    let out_dir = a.first().map(String::as_str);
    if let Some(d) = out_dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{d}: {e}"))?;
    }
    for (body, name, f0) in VOICES {
        println!("\n{name}: F1 / F2 (Hz) of the first vowel");
        let modern: Vec<String> = ["하", "허", "호"].iter().map(|w| {
            let (plan, out, _) = say(word_syllables(w), body, name, f0);
            let (f1, f2) = first_vowel_formants(&plan, &out);
            format!("{w} {f1:.0}/{f2:.0}")
        }).collect();
        println!("  modern     {}", modern.join("   "));
        for (k, h) in ARAE_A.iter().enumerate() {
            let mut row = Vec::new();
            for w in MIDDLE_WORDS {
                let syls = word_syllables_with(w, &Options { arae_a: h.ipa, dialect: Dialect::Middle, ..Default::default() });
                let ipa: String = syls.iter().map(|s| s.onset.iter().map(|o| o.ipa.as_str()).chain([s.nucleus.ipa.as_str()]).chain(s.coda.iter().map(|c| c.ipa.as_str())).collect::<String>()).collect::<Vec<_>>().join(".");
                let (plan, out, _) = say(syls, body, name, f0);
                let (f1, f2) = first_vowel_formants(&plan, &out);
                row.push(format!("{w} [{ipa}] {f1:.0}/{f2:.0}"));
                if let Some(d) = out_dir {
                    let path = format!("{d}/{w}-{}-{name}.wav", (b'A' + k as u8) as char);
                    wav::write(&path, &out, FS as u32, ctx.float).map_err(|e| format!("{path}: {e}"))?;
                }
            }
            println!("  {} ㆍ=[{}] {} ({}): {}", (b'A' + k as u8) as char, h.ipa, h.label, h.source.unwrap_or("source not recorded"), row.join("   "));
        }
    }
    if let Some(d) = out_dir {
        let (body, name, f0) = VOICES[0];
        for (word, label) in [("ᄭᅮᆷ", "kkum"), ("ᄯᆞᄅᆞᆷ", "ttarom")] {
            for (siot_tense, reading) in [(false, "cluster"), (true, "fortis")] {
                let options = Options { dialect: Dialect::Middle, siot_tense, ..Default::default() };
                let (_, out, _) = say(word_syllables_with(word, &options), body, name, f0);
                let path = format!("{d}/{label}-{reading}.wav");
                wav::write(&path, &out, FS as u32, ctx.float).map_err(|e| format!("{path}: {e}"))?;
            }
        }
    }
    Ok(())
}

/// First harmonic over the strongest peak between 2 and 4 kHz (dB)
fn h1_a3(x: &[f64], f0: f64) -> f64 {
    let db = power_spectrum_db(x, &Fft::new(x.len()), &hann(x.len()));
    let df = FS / x.len() as f64;
    let max = |a: f64, b: f64| db[(a / df) as usize..=(b / df) as usize].iter().copied().fold(f64::MIN, f64::max);
    max(0.85 * f0, 1.15 * f0) - max(2000.0, 4000.0)
}

fn st(f: f64, over: f64) -> f64 {
    12.0 * log2(f / over)
}

fn mark(ok: bool) -> &'static str {
    if ok { "ok" } else { "MISS" }
}

pub fn run(ctx: &Ctx, a: &[String]) -> Result<(), String> {
    let out_dir = a.first().map(String::as_str);
    if let Some(d) = out_dir {
        std::fs::create_dir_all(d).map_err(|e| format!("{d}: {e}"))?;
    }
    let (mut checks, mut misses) = (0, 0);
    let mut check = |ok: bool| {
        checks += 1;
        if !ok {
            misses += 1;
        }
        mark(ok)
    };
    for (body, name, f0) in VOICES {
        println!("\n{name}            VOT ms           onset F0 (st over lax)      H1-A3 dB (lax > asp > tense)");
        for (place, lax, asp, tense) in SETS {
            let words: Vec<&str> = [Some(lax), asp, Some(tense)].into_iter().flatten().collect();
            let m: Vec<Measured> = words.iter().map(|w| measure(w, body, name, f0)).collect();
            if let Some(d) = out_dir {
                for (w, x) in words.iter().zip(&m) {
                    let path = format!("{d}/{w}-{name}.wav");
                    wav::write(&path, &x.out, FS as u32, ctx.float).map_err(|e| format!("{path}: {e}"))?;
                }
            }
            let (l, t) = (&m[0], m.last().unwrap());
            let a = asp.map(|_| &m[1]);
            let (vot, f0s) = match a {
                Some(a) => {
                    let (sa, stt) = (st(a.f0, l.f0), st(t.f0, l.f0));
                    (
                        format!("lax {:.0} {} asp {:.0} {} tense {:.0} {}", l.vot, check((50.0..=65.0).contains(&l.vot)), a.vot, check((57.0..=75.0).contains(&a.vot)), t.vot, check((16.0..=24.0).contains(&t.vot))),
                        format!("asp {sa:+.1} {} tense {stt:+.1} {}", check(sa > 3.0), check(stt > 3.0)),
                    )
                }
                // ㅅ/ㅆ: the aspiration after the frication (ㅅ 29-78 ms before [a], ㅆ 9-14; Chang
                // 2007, Yoon 2002); ㅆ starts high as the tense stops do (ㅅ's own onset F0 is
                // unsettled, so it is the reference, not checked)
                None => {
                    let stt = st(t.f0, l.f0);
                    (
                        format!("ㅅ asp {:.0} {} ㅆ asp {:.0} {}", l.vot, check((45.0..=75.0).contains(&l.vot)), t.vot, check(t.vot <= 20.0)),
                        format!("tense {stt:+.1} {}", check(stt > 1.5)),
                    )
                }
            };
            let tilts = m.iter().map(|x| format!("{:.1}", x.tilt)).collect::<Vec<_>>().join(" / ");
            let lowest = m.iter().all(|x| t.tilt <= x.tilt);
            let lax_top = m.iter().all(|x| l.tilt >= x.tilt);
            println!("{:<10} {:<40} {:<26} {tilts}  tense lowest {}  lax highest {}", format!("{} {}", words.join(""), place), vot, f0s, check(lowest), check(lax_top));
        }
        // readings whose stops keep apart by VOT: Gyeongsang lax 38, aspirated 104 (이현정·Jongman
        // 2012); the north lax 21.9, aspirated 81.1 (Jung, Oh & Han 2025)
        for (dialect, label, lax, asp) in [(Dialect::Gyeongsang, "gyeongsang", 30.0..=46.0, 95.0..=113.0), (Dialect::Northeast, "northeast", 15.0..=29.0, 73.0..=89.0)] {
            let o = Options { dialect, ..Default::default() };
            let [l, a, s] = ["바", "파", "사"].map(|w| measure_syllables(word_syllables_with(w, &o), body, name, f0));
            // Gyeongsang's ㅅ: Busan 38 against Seoul 48 ms (Holliday 2012), so ~0.79 of Seoul's 60
            let siot = if dialect == Dialect::Gyeongsang { 40.0..=54.0 } else { 52.0..=68.0 };
            println!("바파사 {label:<10} lax {:.0} {} asp {:.0} {} ㅅ asp {:.0} {}", l.vot, check(lax.contains(&l.vot)), a.vot, check(asp.contains(&a.vot)), s.vot, check(siot.contains(&s.vot)));
            if dialect == Dialect::Gyeongsang {
                // a high-low word's stop is shorter than a low-high one's (lax 41.28 vs 49.06 ms,
                // aspirated 70.49 vs 76.06; 서민정 2026)
                let v = |w: &str| measure_syllables(word_syllables_with(w, &o), body, name, f0).vot;
                let (hl, lh, phl, plh) = (v("바\u{301}다"), v("바다\u{301}"), v("파\u{301}다"), v("파다\u{301}"));
                println!("바다 HL/LH  lax {hl:.0}/{lh:.0} {} asp {phl:.0}/{plh:.0} {}", check(hl < lh - 3.0), check(phl < plh - 3.0));
                // a word's F0 range (lowest to highest, semitones): older South Kyungsang HL 3.29,
                // HH 3.59, LH 6.91 (Lee & Jongman 2015, Table 5)
                let range = |w: &str| {
                    let (plan, out, _) = say(word_syllables_with(w, &o), body, name, f0);
                    let pitches: Vec<f64> = plan.segments.iter().filter(|s| s.kind == "vowel").flat_map(|v| {
                        let n = ((v.t1 - v.t0 - 0.04) / 0.01).max(0.0) as usize;
                        (0..=n).map(move |k| v.t0 + 0.02 + k as f64 * 0.01)
                    }).filter_map(|t| {
                        let mid = (t * FS) as usize;
                        let fr: Vec<f64> = out[mid - 512..mid + 512].iter().map(|x| *x as f64).collect();
                        Some(yin_pitch(&fr, FS, YinOpts { min_f0: Some(60.0), max_f0: Some(500.0), ..Default::default() }).f0).filter(|p| *p > 0.0)
                    }).collect();
                    let (lo, hi) = pitches.iter().fold((f64::MAX, 0.0f64), |(a, b), &p| (a.min(p), b.max(p)));
                    st(hi, lo)
                };
                let (rhl, rhh, rlh) = (range("바\u{301}다"), range("바\u{301}다\u{301}"), range("바다\u{301}"));
                println!("바다 F0 range st  HL {rhl:.1} {} HH {rhh:.1} {} LH {rlh:.1} {}", check((2.3..=4.3).contains(&rhl)), check((2.6..=4.6).contains(&rhh)), check((5.9..=7.9).contains(&rlh)));
            }
        }
        // sung (Sing's scaled voice onset times): pop singing measures ㅂ 27.7, ㅍ 32.7, ㅃ 10.8 ms
        // (이철희·한경훈 2020)
        let [l, a, t] = ["바", "파", "빠"].map(|w| measure_sung(w, body, name, f0));
        println!("바파빠 sung  lax {:.0} {} asp {:.0} {} tense {:.0} {}", l.vot, check((20.0..=35.0).contains(&l.vot)), a.vot, check((25.0..=40.0).contains(&a.vot) && a.vot > l.vot), t.vot, check(t.vot < l.vot));
    }
    println!("\n{} of {checks} checks met", checks - misses);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_stops_order_by_vot_and_tense_presses_its_vowel() {
        let (body, name, f0) = VOICES[0];
        let [l, a, t] = ["가", "카", "까"].map(|w| measure(w, body, name, f0));
        assert!(t.vot < l.vot && l.vot < a.vot, "{} {} {}", l.vot, a.vot, t.vot);
        assert!(t.tilt < l.tilt, "{} {}", l.tilt, t.tilt);
        assert!(l.f0 > 0.0 && t.f0 > l.f0);
    }

    #[test]
    fn ssangsiot_voices_soon_and_periodic_after_its_frication() {
        // Seoul ㅆ: 9-14 ms before the voice (Chang 2007); was 35. Closing the folds sooner at the
        // stops' fold tension left the male voice without pitch 25 ms in
        for (body, name, f0) in VOICES {
            let [s, ss] = ["사", "싸"].map(|w| measure(w, body, name, f0));
            assert!(ss.vot < 22.0 && s.vot > 45.0, "{name}: ㅅ {} ㅆ {}", s.vot, ss.vot);
            assert!(ss.f0 > 0.0 && ss.tilt < s.tilt, "{name}: f0 {} tilt {} vs {}", ss.f0, ss.tilt, s.tilt);
        }
    }

    #[test]
    fn sung_stops_aspirate_briefly_and_keep_their_order() {
        // pop singing: ㅂ 27.7, ㅍ 32.7, ㅃ 10.8 ms (이철희·한경훈 2020); at a 0 floor they all sat at ~41
        let (body, name, f0) = VOICES[0];
        let [l, a, t] = ["바", "파", "빠"].map(|w| measure_sung(w, body, name, f0));
        assert!(t.vot < l.vot && l.vot < a.vot && a.vot < 36.0, "{} {} {}", l.vot, a.vot, t.vot);
    }
}
