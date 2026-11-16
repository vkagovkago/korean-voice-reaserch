//! Port of tests/syllables.test.ts.
use chimera::phonetics::Segment;
use chimera::phonetics::english::arpabet_to_segments;
use chimera::phonetics::syllables::{lyrics_to_syllables, split_spelling, syllabify_segments};

// a few CMUdict entries, enough to test without loading the dictionary
fn dict(w: &str) -> Option<&'static str> {
    Some(match w {
        "twinkle" => "T W IH1 NG K AH0 L",
        "little" => "L IH1 T AH0 L",
        "star" => "S T AA1 R",
        "how" => "HH AW1",
        "i" => "AY1",
        "wonder" => "W AH1 N D ER0",
        "what" => "W AH1 T",
        "you" => "Y UW1",
        "are" => "AA1 R",
        "extra" => "EH1 K S T R AH0",
        "of" => "AH1 V",
        _ => return None,
    })
}
fn word(w: &str, _reduce: bool) -> Vec<Segment> {
    dict(w).map(arpabet_to_segments).unwrap_or_default()
}
fn ipa(xs: &[Segment]) -> String {
    xs.iter().map(|x| x.ipa.as_str()).collect()
}

#[test]
fn maximal_onset_with_legal_english_clusters() {
    let tw = syllabify_segments(&word("twinkle", false));
    let got: Vec<[String; 3]> = tw.iter().map(|s| [ipa(&s.onset), s.nucleus.ipa.clone(), ipa(&s.coda)]).collect();
    assert_eq!(got, [["tw", "ɪ", "ŋ"].map(String::from), ["k", "ə", "l"].map(String::from)]);
    // "extra": k + str → ek-stra (str is a legal onset)
    let ex = syllabify_segments(&word("extra", false));
    assert_eq!(ex.iter().map(|s| ipa(&s.onset)).collect::<Vec<_>>(), ["", "stɹ"]);
    assert_eq!(ipa(&ex[0].coda), "k");
}

#[test]
fn spelling_splits_to_match() {
    assert_eq!(split_spelling("twinkle", 2), ["twin", "kle"]);
    assert_eq!(split_spelling("little", 2), ["lit", "tle"]);
    assert_eq!(split_spelling("wonder", 2), ["won", "der"]);
    assert_eq!(split_spelling("are", 1), ["are"]);
}

#[test]
fn lyrics_words_melisma_phrase_ends_unreduced_function_words() {
    let syl = lyrics_to_syllables("Twinkle little star, how I - wonder", &word);
    assert_eq!(syl.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["Twin", "kle", "lit", "tle", "star", "how", "I", "~", "won", "der"]);
    assert!(syl[4].phrase_end);
    assert_eq!(syl[7].melisma, Some(true));
    assert_eq!(syl[7].nucleus.ipa, "aɪ");
    assert!(syl[1].word_end && !syl[0].word_end);
}

/// Each syllable as onset+nucleus+coda IPA, joined with "."; a stop's aspiration, when its front end
/// decides it, as ʰ (aspirated) or ˭ (unaspirated)
fn sung(text: &str) -> String {
    // aspiration with its length when the front end gives one: ʰ45 (45 ms), ʰ90
    let marked = |segs: &[Segment]| {
        segs.iter()
            .map(|s| {
                let asp = match (s.aspirated, s.vot) {
                    (Some(true), Some(v)) => format!("ʰ{}", (v * 1000.0).round()),
                    (Some(true), None) => "ʰ".into(),
                    (Some(false), _) => "˭".into(),
                    (None, _) => String::new(),
                };
                format!("{}{asp}", s.ipa)
            })
            .collect::<String>()
    };
    lyrics_to_syllables(text, &word).iter().map(|s| format!("{}{}{}", marked(&s.onset), s.nucleus.ipa, marked(&s.coda))).collect::<Vec<_>>().join(".")
}

#[test]
fn hangul_sings_one_syllable_per_block_with_korean_sound_rules() {
    assert_eq!(sung("빵야"), "p˭aŋ.ja"); // ㅇ coda stays; tense ㅃ is voiceless and unaspirated
    assert_eq!(sung("먹어"), "mʌ.gʌ"); // a coda moves onto a silent ㅇ, voiced between vowels
    assert_eq!(sung("좋아"), "tɕʰ45o.a"); // ㅎ drops before a vowel; ㅈ is alveolo-palatal
    assert_eq!(sung("좋다"), "tɕʰ45o.tʰ57a"); // ㅎ + ㄷ aspirates
    assert_eq!(sung("국물"), "kʰ45uŋ.mul"); // nasalisation
    assert_eq!(sung("신라"), "ɕil.la"); // ㄴ + ㄹ is [ll]; ㅅ before i is [ɕ]
    assert_eq!(sung("없는"), "ʌm.nɯn"); // a two-letter coda simplifies, then nasalises
    assert_eq!(sung("학교"), "hak.k˭jo"); // tensing after an obstruent coda
    assert_eq!(sung("사랑해"), "sa.ɾaŋ.ɦɛ");
    assert_eq!(sung("닭"), "tʰ45ak");
    assert_eq!(sung("의사"), "ɰi.sa");
    // after a dropped ㅎ the consonant before it moves onto the silent ㅇ
    assert_eq!(sung("싫어"), "ɕi.ɾʌ"); // [시러]
    assert_eq!(sung("많아"), "ma.na"); // [마나]
    assert_eq!(sung("괜찮아"), "kʰ45wɛn.tɕʰ57a.na"); // [괜차나]
    assert_eq!(sung("싫니"), "ɕil.li"); // ㅀ before ㄴ: [실리]
    assert_eq!(sung("않는"), "an.nɯn"); // ㄶ before ㄴ: [안는]
    // ㄷ ㅌ before 이 palatalise
    assert_eq!(sung("같이"), "kʰ45a.tɕʰ57i"); // [가치]
    assert_eq!(sung("굳이"), "kʰ45u.dʑi"); // [구지]
}

#[test]
fn korean_stops_stay_three_ways_apart_word_initially() {
    // word-initially all three are voiceless: aspirated long, lax briefly, tense not at all
    // (불 fire, 풀 grass, 뿔 horn); they used to sing as pul, pul and bul
    assert_eq!(sung("불 풀 뿔"), "pʰ45ul.pʰ57ul.p˭ul");
    assert_eq!(sung("달 탈 딸"), "tʰ45al.tʰ57al.t˭al");
    assert_eq!(sung("가 카 까"), "kʰ45a.kʰ57a.k˭a");
    // inside a word: lax voiced between vowels, the other two as word-initially
    assert_eq!(sung("아기 아키 아끼"), "a.gi.a.kʰ57i.a.k˭i");
}

#[test]
fn lax_and_aspirated_stops_plan_different_aspiration() {
    use chimera::phonetics::planner::{PlanOptions, plan_utterance};
    // how long the folds stay apart after the release: the aspiration gesture's end
    let aspiration_end = |text: &str| {
        let s = &lyrics_to_syllables(text, &word)[0];
        let segs: Vec<Segment> = s.onset.iter().cloned().map(|mut c| { c.dur = None; c }).chain([Segment::new(&s.nucleus.ipa)]).chain(s.coda.iter().cloned()).collect();
        let plan = plan_utterance(&segs, &PlanOptions::default());
        plan.gestures.iter().find(|g| g.label.as_ref().map(|l| l.as_str()) == Some("aspiration")).map(|g| g.t1).unwrap()
    };
    let (lax, aspirated) = (aspiration_end("불"), aspiration_end("풀"));
    // the folds stay apart 12 ms longer for 풀 (57 against 45): the difference its front end asked
    // for, which the render turns into 70 against 57 ms of VOT (`chimera diagnose korean`)
    assert!((aspirated - lax - 0.012).abs() < 1e-9, "lax {lax}, aspirated {aspirated}");
}

#[test]
fn text_after_a_letter_no_front_end_reads_is_kept() {
    // accented Latin letters aren't English's: they are skipped, the rest of the word still sings
    // (as before the router). A front end must never swallow what follows.
    let g2p = chimera::phonetics::english::EnglishG2P::new(); // letter-to-sound: reads any word
    let texts: Vec<String> = lyrics_to_syllables("Zürich über", &g2p).into_iter().map(|s| s.text).collect();
    assert!(texts.iter().any(|t| t == "rich") && texts.iter().any(|t| t == "ber"), "{texts:?}");
}

#[test]
fn hangul_keeps_its_spelling_and_mixes_with_english() {
    let syls = lyrics_to_syllables("twinkle 빵야 빵야, star", &word);
    let shown: Vec<(&str, bool, bool)> = syls.iter().map(|s| (s.text.as_str(), s.word_end, s.phrase_end)).collect();
    assert_eq!(
        shown,
        [("twin", false, false), ("kle", true, false), ("빵", false, false), ("야", true, false), ("빵", false, false), ("야", true, true), ("star", true, false)]
    );
}

#[test]
fn hand_written_syllables_may_leave_out_word_and_phrase_ends() {
    let s: chimera::phonetics::Syllable = serde_json::from_str(r#"{"text": "풀", "onset": [{"ipa": "p", "aspirated": true, "vot": 0.07}], "nucleus": {"ipa": "u"}, "coda": [{"ipa": "l"}]}"#).unwrap();
    assert!(!s.word_end && !s.phrase_end);
    assert_eq!(s.onset[0].vot, Some(0.07));
}

#[test]
fn korean_stops_close_for_tense_longest_lax_shortest() {
    // 2026 notes: closure tense 123 ms > aspirated 84 > lax 48 (VCV); ㅎ between vowels is brief
    let dur = |t: &str| lyrics_to_syllables(t, &word)[0].onset[0].dur.unwrap();
    assert!(dur("뿔") > dur("풀") && dur("풀") > dur("불"));
    assert!(lyrics_to_syllables("아하", &word)[1].onset[0].dur.unwrap() < 0.06);
}

#[test]
fn vowel_pitch_after_korean_stops_is_high_for_tense_and_aspirated_low_for_lax() {
    use chimera::phonetics::planner::{PlanOptions, plan_utterance};
    use chimera::control::score::ControlKey;
    // 2026 notes: the vowel after 뿔/풀 starts ~5 semitones above the one after 불 (speech, no melody)
    let pitch = |text: &str| {
        let segs: Vec<Segment> = lyrics_to_syllables(text, &word).iter().flat_map(|s| s.onset.iter().cloned().chain([Segment::new(&s.nucleus.ipa)]).chain(s.coda.iter().cloned())).collect();
        let plan = plan_utterance(&segs, &PlanOptions::default());
        plan.gestures.iter().find(|g| g.label.as_ref().map(|l| l.as_str()) == Some("pitch")).and_then(|g| g.targets.get(ControlKey::F0)).unwrap()
    };
    let (lax, asp, tense) = (pitch("불 불"), pitch("풀 풀"), pitch("뿔 뿔"));
    assert!(asp > lax * 1.25 && tense > lax * 1.25, "{lax} {asp} {tense}");
}

#[test]
fn korean_compounds_and_sino_korean_words_read_from_their_parts() {
    // ㄴ insertion and tensing need the word's parts: a short list, the letters stay as written
    assert_eq!(sung("솜이불"), "som.ni.bul");
    let g = lyrics_to_syllables("갈등", &word);
    assert_eq!(g.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(), ["갈", "등"]);
    assert_eq!(g[1].onset[0].aspirated, Some(false));
}

#[test]
fn korean_tense_onsets_stiffen_the_folds() {
    // ㄲ ㄸ ㅃ ㅉ ㅆ are fortis, written or made by a rule (국밥 [국빱]); lax and aspirated ones are not
    let fortis = |t: &str| lyrics_to_syllables(t, &word).iter().map(|s| s.onset.first().and_then(|o| o.fortis) == Some(true)).collect::<Vec<_>>();
    assert_eq!(fortis("까 따 빠 짜 싸"), [true; 5]);
    assert_eq!(fortis("가 카 사 하 나"), [false; 5]);
    assert_eq!(fortis("국밥"), [false, true]);
}

#[test]
fn korean_lax_stops_start_their_vowel_breathy_word_initially() {
    // voiceless lax ㄱ ㄷ ㅂ ㅈ (word-initially) are breathy; voiced ones between vowels, aspirated and tense are not
    let breathy = |t: &str| lyrics_to_syllables(t, &word).iter().map(|s| s.onset.first().and_then(|o| o.breathy) == Some(true)).collect::<Vec<_>>();
    assert_eq!(breathy("가 다 바 자"), [true; 4]);
    assert_eq!(breathy("카 까 사"), [false; 3]);
    assert_eq!(breathy("아기"), [false, false]);
}

#[test]
fn old_hangul_reads_as_conjoining_jamo_with_its_archaic_letters() {
    use chimera::phonetics::korean::{ARAE_A, Options, word_syllables_with};
    // ᄒᆞᆫ 'one', ᄆᆞᅀᆞᆷ 'heart', ᅌᅵ, ᄫᅡ, and ㆆ as an initial and a final; ㆍ by its first hypothesis
    assert_eq!(sung("ᄒᆞᆫ ᄆᆞᅀᆞᆷ"), "hʌn.mʌ.zʌm");
    assert_eq!(sung("ᅌᅵ ᄫᅡ ᅙᅡ 사ᇹ"), "ŋi.βa.ʔa.saʔ");
    // a final ㅿ moves onto a vowel after it and is [z]; ㆎ is ㆍ with an offglide
    assert_eq!(sung("바ᇫ이"), "pʰ45a.zi");
    assert_eq!(sung("ᄫᆡ"), "βʌj"); // ㆎ: ㆍ with [j] after it
    // each block keeps its jamo as its text, tone dots included
    let s = lyrics_to_syllables("ᄒᆞᆫ〮", &word);
    assert_eq!(s[0].text, "ᄒᆞᆫ〮");
    // the other hypotheses for ㆍ
    let vowel = |ipa| word_syllables_with("ᄃᆞᆯ", &Options { arae_a: ipa, ..Default::default() })[0].nucleus.ipa.clone();
    assert_eq!(ARAE_A.map(|h| vowel(h.ipa)), ["ʌ", "ɐ", "ɔ"]);
    // modern Hangul is read as before
    assert_eq!(sung("한 마음"), "han.ma.ɯm");
}

#[test]
fn gyeongsang_merges_eo_and_eu_says_ss_as_s_and_raises_marked_syllables() {
    use chimera::control::score::ControlKey;
    use chimera::phonetics::korean::{Dialect, Options, word_syllables_with};
    use chimera::phonetics::planner::{PlanOptions, plan_utterance};
    let gs = Options { dialect: Dialect::Gyeongsang, ..Default::default() };
    let read = |w: &str, o: &Options| word_syllables_with(w, o);
    let vowels = |w: &str, o: &Options| read(w, o).iter().map(|s| s.nucleus.ipa.clone()).collect::<Vec<_>>().join(".");
    // ㅓ ㅡ (and ㅕ) are one vowel [ə]; Seoul keeps them apart
    assert_eq!(vowels("거그겨", &gs), "ə.ə.ə");
    assert_eq!(vowels("거그", &Options::default()), "ʌ.ɯ");
    // 쌀 is said as 살: no fortis
    assert_eq!(read("쌀", &gs)[0].onset[0], read("살", &gs)[0].onset[0]);
    assert_eq!(read("쌀", &Options::default())[0].onset[0].fortis, Some(true));
    // an acute marks the high syllable; Seoul ignores it, and the text keeps it
    let stress = |w: &str, o: &Options| read(w, o).iter().map(|s| s.nucleus.stress == Some(1)).collect::<Vec<_>>();
    assert_eq!(stress("가\u{301}지", &gs), [true, false]);
    assert_eq!(stress("가지\u{301}", &gs), [false, true]);
    assert_eq!(stress("가\u{301}지", &Options::default()), [false, false]);
    assert_eq!(read("가\u{301}지", &gs)[0].text, "가\u{301}");
    // and the planner puts the pitch peak on it
    let peak_on_first = |w: &str| {
        let segs: Vec<Segment> = read(w, &gs).iter().flat_map(|s| s.onset.iter().cloned().chain([s.nucleus.clone()]).chain(s.coda.iter().cloned())).collect();
        let plan = plan_utterance(&segs, &PlanOptions::default());
        let f0s: Vec<f64> = plan.gestures.iter().filter(|g| g.label.as_ref().map(|l| l.as_str()) == Some("pitch")).filter_map(|g| g.targets.get(ControlKey::F0)).collect();
        f0s[0] > f0s[1]
    };
    assert!(peak_on_first("가\u{301}지") && !peak_on_first("가지\u{301}"));
    // the lyric router keeps the mark inside the word
    assert_eq!(lyrics_to_syllables("가\u{301}지", &word).len(), 2);
}

#[test]
fn speech_keeps_a_korean_accent_mark_in_its_word_only_after_hangul() {
    use chimera::phonetics::english::EnglishG2P;
    use chimera::phonetics::korean::Dialect;
    let mut g = EnglishG2P::new();
    g.korean.dialect = Dialect::Gyeongsang;
    let stressed = |t: &str| g.to_phrases(t)[0].segments.iter().filter(|s| s.stress == Some(1)).count();
    assert_eq!(stressed("가\u{301}지"), 1);
    assert_eq!(g.to_phrases("가\u{301}지")[0].words, ["가\u{301}지"]);
}

#[test]
fn korean_words_in_speech_keep_their_own_l_and_d() {
    // English's allophones ran over Hangul too: 살 ended in a dark [ɫ], 바다's voiced [d] was tapped
    use chimera::phonetics::english::EnglishG2P;
    let g = EnglishG2P::new();
    let ipa = |t: &str| g.to_phrases(t)[0].segments.iter().map(|s| s.ipa.as_str()).collect::<String>();
    assert_eq!(ipa("살"), "sal");
    assert_eq!(ipa("바다"), "pada");
    // English keeps them, next to Korean too
    let g = EnglishG2P::new();
    assert!(g.to_phrases("cool 살")[0].segments.iter().any(|s| s.ipa == "ɫ"));
}

#[test]
fn sung_lyrics_read_hangul_in_the_dialect() {
    use chimera::phonetics::korean::{Dialect, Options};
    use chimera::phonetics::syllables::lyrics_to_syllables_in;
    let gs = Options { dialect: Dialect::Gyeongsang, ..Default::default() };
    let vowels = |o: Options| lyrics_to_syllables_in("거그 쌀", &word, o).iter().map(|s| s.nucleus.ipa.clone()).collect::<Vec<_>>().join(".");
    assert_eq!(vowels(gs), "ə.ə.a");
    assert_eq!(vowels(Options::default()), "ʌ.ɯ.a");
    assert_eq!(lyrics_to_syllables_in("쌀", &word, gs)[0].onset[0].fortis, None);
}

fn read_as(text: &str, o: chimera::phonetics::korean::Options) -> String {
    chimera::phonetics::korean::word_syllables_with(text, &o)
        .iter()
        .map(|s| s.onset.iter().map(|x| x.ipa.as_str()).chain([s.nucleus.ipa.as_str()]).chain(s.coda.iter().map(|x| x.ipa.as_str())).collect::<String>())
        .collect::<Vec<_>>()
        .join(".")
}

#[test]
fn the_15th_century_reading_has_its_vowels_diphthongs_eight_finals_and_tones() {
    use chimera::phonetics::korean::{Dialect, Options, word_syllables_with};
    let mk = Options { dialect: Dialect::Middle, ..Default::default() };
    // ㅓ [ə] ㅡ [ɨ], beside ㆍ [ʌ]; ㅐ ㅔ ㅚ ㅟ ㅢ end in [j]
    assert_eq!(["ᄒᆞᆫ", "어", "으"].map(|w| read_as(w, mk)), ["hʌn", "ə", "ɨ"]);
    assert_eq!(["개", "게", "괴", "귀", "긔"].map(|w| read_as(w, mk)), ["kaj", "kəj", "koj", "kuj", "kɨj"]);
    assert_eq!(read_as("ᄀᆡ", mk), "kʌj");
    // a final ㅅ stays [s] (옷 [os], not modern [ot])
    assert_eq!(read_as("옷", mk), "os");
    assert_eq!(read_as("옷", Options::default()), "ot");
    // tone dots: one high, two high and long, none low
    let n = |t: &str| word_syllables_with(t, &mk).iter().map(|s| (s.nucleus.stress, s.nucleus.dur)).collect::<Vec<_>>();
    assert_eq!(n("ᄆᆞᆯ〮"), [(Some(1), None)]);
    assert_eq!(n("ᄆᆞᆯ〯"), [(Some(1), Some(0.25))]);
    assert_eq!(n("ᄆᆞᆯ"), [(None, None)]);
    // modern Seoul is as before
    assert_eq!(read_as("개", Options::default()), "kʰ45ɛ".replace("ʰ45", ""));
}

#[test]
fn old_hangul_clusters_read_as_clusters_or_the_siot_series_as_tense() {
    use chimera::phonetics::korean::{Options, word_syllables_with};
    // ᄠᅳᆮ (뜻), ᄢᅢ (깨), ᄡᆞᆯ (쌀), ᄭᅮᆷ (꿈)
    assert_eq!(read_as("ᄠᅳᆮ", Options::default()), "ptɯt");
    assert_eq!(read_as("ᄢᅢ", Options::default()), "pskɛ");
    assert_eq!(read_as("ᄡᆞᆯ", Options::default()), "psʌl");
    assert_eq!(read_as("ᄭᅮᆷ", Options::default()), "skum");
    let tense = Options { siot_tense: true, ..Default::default() };
    assert_eq!(read_as("ᄭᅮᆷ", tense), "kum");
    assert_eq!(word_syllables_with("ᄭᅮᆷ", &tense)[0].onset[0].fortis, Some(true));
    assert_eq!(read_as("ᄢᅢ", tense), "pkɛ");
    // a stop after a cluster doesn't aspirate
    assert!(word_syllables_with("ᄭᅮᆷ", &Options::default())[0].onset.iter().all(|s| s.aspirated != Some(true)));
}

#[test]
fn jeju_reads_arae_a_as_a_low_back_rounded_vowel() {
    use chimera::phonetics::korean::{Dialect, Options};
    let jeju = Options { dialect: Dialect::Jeju, ..Default::default() };
    assert_eq!(read_as("ᄆᆞᆯ", jeju), "mɒl");
    assert_eq!(read_as("ᄋᆢ덥", jeju), "jɒ.dʌp");
    // the rest is modern
    assert_eq!(read_as("말", jeju), "mal");
}
