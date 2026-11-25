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
    assert_eq!(sung("신라"), "ɕʰ5il.la"); // ㄴ + ㄹ is [ll]; ㅅ before i is [ɕ], briefly aspirated
    assert_eq!(sung("없는"), "ʌm.nɯn"); // a two-letter coda simplifies, then nasalises
    assert_eq!(sung("학교"), "hak.k˭jo"); // tensing after an obstruent coda
    assert_eq!(sung("사랑해"), "sʰ45a.ɾaŋ.ɦɛ");
    assert_eq!(sung("닭"), "tʰ45ak");
    assert_eq!(sung("의사"), "ɰi.sʰ45a");
    // after a dropped ㅎ the consonant before it moves onto the silent ㅇ
    assert_eq!(sung("싫어"), "ɕʰ5i.ɾʌ"); // [시러]
    assert_eq!(sung("많아"), "ma.na"); // [마나]
    assert_eq!(sung("괜찮아"), "kʰ45wɛn.tɕʰ57a.na"); // [괜차나]
    assert_eq!(sung("싫니"), "ɕʰ5il.li"); // ㅀ before ㄴ: [실리]
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
    // closure tense 137 ms > aspirated 111 > lax 77 in Seoul (강현숙·김태경 2020); ㅎ between vowels is brief
    let dur = |t: &str| lyrics_to_syllables(t, &word)[0].onset[0].dur.unwrap();
    assert!(dur("뿔") > dur("풀") && dur("풀") > dur("불"));
    assert!(lyrics_to_syllables("아하", &word)[1].onset[0].dur.unwrap() < 0.06);
}

#[test]
fn korean_siot_is_aspirated_and_ssangsiot_is_longer() {
    // ㅅ [sʰ] aspirates into its vowel (long before [a], brief as [ɕ] before [i]); ㅆ doesn't, its
    // frication is longer, and it alone is fortis (Chang 2007)
    let onset = |t: &str| lyrics_to_syllables(t, &word)[0].onset[0].clone();
    let (sa, si, ssa) = (onset("사"), onset("시"), onset("싸"));
    assert_eq!((sa.aspirated, si.aspirated, ssa.aspirated), (Some(true), Some(true), None));
    assert!(sa.vot.unwrap() > 0.03 && si.vot.unwrap() < 0.015, "{:?} {:?}", sa.vot, si.vot);
    assert!(ssa.dur.unwrap() > 0.14 && sa.dur.is_none());
    assert_eq!((sa.fortis, ssa.fortis), (None, Some(true)));
}

#[test]
fn korean_word_initial_l_taps_longer_than_between_vowels() {
    // 크로스비·서윤정 2025: a word-initial tap is 27 ms; between vowels the planner's 20 ms flap
    let syls = lyrics_to_syllables("라라", &word);
    assert_eq!(syls[0].onset[0].dur, Some(0.027));
    assert_eq!(syls[1].onset[0].dur, None);
}

#[test]
fn vowel_pitch_after_korean_stops_is_high_for_tense_and_aspirated_low_for_lax() {
    use chimera::phonetics::planner::{PlanOptions, plan_utterance};
    use chimera::control::score::ControlKey;
    // the vowel after 뿔/풀 starts ~5 semitones above the one after 불 (Beckman et al. 2014; speech, no melody)
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
    assert_eq!(sung("솜이불"), "sʰ45om.ni.bul");
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
    assert_eq!(sung("ᅌᅵ ᄫᅡ ᅙᅡ 사ᇹ"), "ŋi.βa.ʔa.sʰ45aʔ");
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
        let first = plan.segments.iter().find(|s| s.kind == "vowel").unwrap();
        let peak = plan.gestures.iter().find(|g| g.targets.get(ControlKey::F0).is_some_and(|f| f > 130.0)).unwrap();
        peak.t0 >= first.t0 && peak.t0 < first.t1
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
fn speech_reads_both_scripts_when_english_and_hangul_touch() {
    use chimera::phonetics::english::EnglishG2P;

    let g = EnglishG2P::new();
    let adjacent = g.to_phrases("Hello안녕하세요 world한글test");
    assert_eq!(adjacent[0].words, ["hello", "안녕하세요", "world", "한글", "test"]);
    assert_eq!(adjacent, g.to_phrases("Hello 안녕하세요 world 한글 test"));
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
    // tone dots: one high, two rising and long, none low
    use chimera::phonetics::Tone;
    let n = |t: &str| word_syllables_with(t, &mk).iter().map(|s| (s.nucleus.tone, s.nucleus.dur)).collect::<Vec<_>>();
    assert_eq!(n("ᄆᆞᆯ〮"), [(Some(Tone::High), None)]);
    assert_eq!(n("ᄆᆞᆯ〯"), [(Some(Tone::Rising), Some(0.25))]);
    assert_eq!(n("ᄆᆞᆯ"), [(Some(Tone::Low), None)]);
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
fn jeju_reads_arae_a_as_a_mid_back_rounded_vowel() {
    use chimera::phonetics::korean::{Dialect, Options};
    let jeju = Options { dialect: Dialect::Jeju, ..Default::default() };
    assert_eq!(read_as("ᄆᆞᆯ", jeju), "mɔl");
    assert_eq!(read_as("ᄋᆢ덥", jeju), "jɔ.dʌp");
    // the rest is modern
    assert_eq!(read_as("말", jeju), "mal");
}

#[test]
fn the_preface_letters_read_vowel_after_vowel_double_ieung_and_final_rieul_hieut() {
    use chimera::phonetics::korean::{Dialect, Options, word_syllables_with};
    let mk = Options { dialect: Dialect::Middle, ..Default::default() };
    // ᄒᆡᅇᅧ: a vowel after a vowel (ᅇ read silent between them) starts its own syllable
    assert_eq!(read_as("ᄒᆡᅇᅧ", mk), "hʌj.jə");
    // ᇙ (ㄹ then ㆆ): said ㄹ (호ᇙ, 몯ᄒᆞᇙ), and in the word its ㆆ tenses the next consonant
    assert_eq!(read_as("호ᇙ", mk), "hol");
    let s = word_syllables_with("호ᇙ가", &mk);
    assert_eq!((s[0].coda.len(), s[1].onset[0].fortis), (1, Some(true)));
    // the 언해본 preface reads with no syllable lost (every block has a vowel)
    let text = "나랏 말ᄊᆞ미 中듀ᇰ國귁에 달아 ᄆᆞᄎᆞᆷ내 제 ᄠᅳ들 시러 펴디 몯ᄒᆞᇙ 노미 하니라";
    let syls = lyrics_to_syllables(text, &word);
    let blocks = text.chars().filter(|c| ('\u{AC00}'..='\u{D7A3}').contains(c) || ('\u{1161}'..='\u{11A7}').contains(c)).count();
    assert_eq!(syls.len(), blocks);
}


#[test]
fn regional_profiles_apply_distinct_pronunciation_rules() {
    use chimera::phonetics::korean::{Dialect, Options};
    let read = |word, name| read_as(word, Options { dialect: Dialect::from_name(name).unwrap(), ..Default::default() });
    assert_eq!(read("개", "central"), "ke");
    assert_eq!(read("의사", "southwest"), "ɯ.sa");
    assert_eq!(read("서으", "southeast"), "sə.ə");
    assert_eq!(read("ᄆᆞᆯ", "jeju"), "mɔl");
    assert_eq!(read("같이", "northwest"), "ka.ti");
    assert_eq!(read("의자", "northeast"), "i.dza");
    assert_eq!(Dialect::from_name("gyeongsang"), Dialect::from_name("southeast"));
    assert_eq!(read("개", "seoul"), "kɛ");
}

#[test]
fn middle_korean_neither_palatalises_nor_fronts_its_dental_sibilants() {
    // 15th century: ㄷ ㅌ before 이 stay (디 not 지), ㅈ ㅊ are dental [ts], and ㅅ before i is [s]
    // (palatalisation came in the 17th-18th centuries; 이기문 1972)
    use chimera::phonetics::korean::{Dialect, Options};
    let mk = Options { dialect: Dialect::Middle, ..Default::default() };
    assert_eq!(read_as("굳이", mk), "ku.di");
    assert_eq!(read_as("같이", mk), "ka.ti");
    assert_eq!(read_as("지", mk), "tsi");
    assert_eq!(read_as("시", mk), "si");
    assert_eq!(read_as("셔", mk), "sjə");
    assert_eq!(read_as("같이", Options::default()), "ka.tɕi");
}

#[test]
fn middle_korean_rises_inside_the_vowel_even_at_the_end_of_a_statement() {
    use chimera::control::score::{ControlKey, activation};
    use chimera::phonetics::english::EnglishG2P;
    use chimera::phonetics::korean::{Dialect, Options};
    use chimera::phonetics::planner::{PlanOptions, plan_utterance};

    let mut g = EnglishG2P::default();
    g.korean = Options { dialect: Dialect::Middle, ..Default::default() };
    let phrase = &g.to_phrases("말〯.")[0];
    let plan = plan_utterance(&phrase.segments, &PlanOptions { intonation: Some(phrase.intonation), ..Default::default() });
    let vowel = plan.segments.iter().find(|s| s.ipa == "a").unwrap();
    let f0_at = |fraction: f64| {
        let at = vowel.t0 + fraction * (vowel.t1 - vowel.t0);
        let (sum, weight) = plan.gestures.iter().filter_map(|g| {
            let hz = g.targets.get(ControlKey::F0)?;
            let weight = activation(g, at) * g.strength;
            Some((hz * weight, weight))
        }).fold((0.0, 0.0), |(sum, weight), (s, w)| (sum + s, weight + w));
        sum / weight
    };
    assert!(f0_at(0.2) < f0_at(0.5) && f0_at(0.5) < f0_at(0.8));
}


#[test]
fn middle_korean_tenses_after_written_rieul_hieuh_across_a_space() {
    use chimera::phonetics::english::EnglishG2P;
    use chimera::phonetics::korean::Dialect;

    let mut g = EnglishG2P::new();
    g.korean.dialect = Dialect::Middle;
    let joined = &g.to_phrases("호ᇙ〮 배〮")[0];
    assert_eq!(joined.words, ["호ᇙ〮", "배〮"]);
    assert!(joined.segments[2].word_end == Some(true));
    assert_eq!(joined.segments[3].ipa, "p");
    assert_eq!(joined.segments[3].fortis, Some(true));
    assert_eq!(joined.segments[3].aspirated, Some(false));
    assert_eq!(g.to_phrases("호ᇙ〮, 배〮")[1].segments[0].fortis, None);
    assert_eq!(g.to_phrases("홀 배")[0].segments[3].fortis, None);
    assert_eq!(g.to_phrases("호ᇙ〮 abc 배〮")[0].segments.iter().rev().nth(2).and_then(|s| s.fortis), None);
    g.korean.dialect = Dialect::Seoul;
    assert_eq!(g.to_phrases("호ᇙ〮 배〮")[0].segments[3].fortis, None);
}

#[test]
fn korean_phrase_edges_and_final_tones_depend_on_the_reading() {
    use chimera::phonetics::Tone;
    use chimera::phonetics::english::EnglishG2P;
    use chimera::phonetics::korean::Dialect;

    let tones = |dialect, text| {
        let mut g = EnglishG2P::new();
        g.korean.dialect = dialect;
        g.to_phrases(text).into_iter().flat_map(|p| p.segments.into_iter().filter_map(|s| s.tone)).collect::<Vec<_>>()
    };
    use Tone::{Falling, High, Low, Rising};
    assert_eq!(tones(Dialect::Central, "바다로 가요?"), [Low, High, High, Low, Rising]);
    assert_eq!(tones(Dialect::Southwest, "바다로 가요."), [Low, High, Low, Low, Falling]);
    assert_eq!(tones(Dialect::Jeju, "코끼리가 와요?"), [High, High, High, Low, Low, Rising]);
    assert_eq!(tones(Dialect::Seoul, "바다로 가요?"), []);
    assert_eq!(tones(Dialect::Southwest, "Hello 바다로 there."), [Low, High, Low]);
}

#[test]
fn kyungsang_accent_keeps_a_double_high_and_places_the_late_low() {
    use chimera::phonetics::Tone;
    use chimera::phonetics::korean::{Dialect, Options, word_syllables_with};
    let o = Options { dialect: Dialect::Gyeongsang, ..Default::default() };
    let tones = |text| word_syllables_with(text, &o).into_iter().map(|s| s.nucleus.tone).collect::<Vec<_>>();
    use Tone::{Accent, AccentHigh, Low};
    assert_eq!(tones("가́나다"), [Some(Accent), None, Some(Low)]);
    assert_eq!(tones("가́나́다"), [Some(Accent), Some(AccentHigh), None]);
    assert_eq!(tones("가나다́"), [Some(Low), Some(Low), Some(Accent)]);
    assert_eq!(tones("가나다"), [None, None, None]);
}
