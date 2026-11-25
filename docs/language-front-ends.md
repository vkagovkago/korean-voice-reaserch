# Language front ends

CHIMERA sings other languages in two halves, the usual split in speech synthesis:

```
lyrics ──► router ──► FRONT END ──► syllables ──► BACK END ──► sound
           (by script)  English: CMUdict         (IPA, with what     the planner, the airway,
                        Korean: Hangul rules      the language        the voice: the same for
                        explicit syllables        decides)            every language
```

**A front end is phonology; the back end is physics.** A front end decides what a word's sounds
are: which phonemes, and whether and how long a stop aspirates. The back end realises whatever
the syllables say and knows no language's rules. When a front end says nothing, the planner keeps
English's defaults, which is why English output doesn't change.

This is the shared design for languages (#40), and the first steps of #32 live in it: the sounds
Korean and Mandarin need (`ɕ ʑ tɕ dʑ tʂ dʐ ɥ`, step 3) and aspiration as the front end's call
(step 1, with #73).

## The built-in front ends

| Script | Front end | Needs | Where |
|---|---|---|---|
| Latin | English | CMUdict (letter-to-sound until it loads) | `phonetics/english.rs`, `syllables.rs` |
| Hangul | Korean | nothing: a block's letters are its sounds | `phonetics/korean.rs` |
| Cyrillic | Russian | nothing: spelling rules, stress marked with an acute ([russian.md](russian.md)) | `phonetics/russian.rs` |

The router (`read_lyrics` in `syllables.rs`) sends each word to the front end whose script it is
in, so a line can mix them (`twinkle 빵야 star`). It also handles what every language shares:
melisma marks (`-`, `~`), phrase punctuation and word ends.

Characters no front end claims are skipped, as before: `Zürich` sings "Z rich". Nothing after them
is lost.

`chimera syllables "<lyrics>"` prints what the front ends read, one JSON syllable per sung note:
the quickest way to check a front end.

## Korean

One Hangul block is one note, and the block keeps its spelling as the note's text. Inside a word
the front end applies standard Seoul sound rules: a coda moves onto a following silent ㅇ (먹어 →
머거), codas neutralise, obstruents nasalise before nasals (국물 → 궁물), ㄴ and ㄹ meet as [ll],
ㅎ aspirates its neighbour (좋다 → 조타), and obstruents tense after an obstruent coda.

Korean's three stops are all voiceless at the start of a word. The front end tells them apart by
how long they aspirate (`vot`, voice onset time):

| | 불 fire | 풀 grass | 뿔 horn |
|---|---|---|---|
| kind | lax | aspirated | tense |
| written | [pʰ], `vot` 45 ms | [pʰ], `vot` 57 ms | [p˭], unaspirated |
| measured in the render | 57 ms | 70 ms | 24 ms (27 for ㄸ, male) |
| Seoul speech | 50-65 ms | 57-75 ms | 16-24 ms |

`chimera diagnose korean [outdir]` measures this (all four places, ㅅ/ㅆ, male and female; with an
outdir it writes each syllable's WAV to listen to): VOT, onset F0 over the lax stop, and the
spectral tilt (H1-A3) 30-50 ms into the voice. 59 of its 60 checks are met; the miss: ㄸ's VOT is
3 ms long in the male voice. ㅅ/ㅆ leave the onset F0 where it is (shown, not checked).

A word-initial lax stop starts its vowel breathy and slack (`breathy` on an aspirated stop: abduction
0.1 and fold tension 0.3 from the end of the aspiration to 80 ms after), so its vowel is the
breathiest of the three (H1-A3 22-25 dB female against 19 after an aspirated stop).

Closure lasts longest for tense stops (120 ms here), then aspirated (85), then lax (50): the order Seoul speech has (137, 111, 77 over three positions, 강현숙·김태경 2020), at running-speech lengths; ㅎ between vowels is brief (it is mostly dropped in real speech).

ㅅ is aspirated [sʰ]: the folds stay apart past its frication, so its vowel starts ~60 ms later (before [a]: 29-78 ms, Chang 2007; before [i], where it is [ɕ], about 7). ㅆ's frication is longer (160 ms against [s]'s ~105: 180-218 against 109-167, Chang 2007), and its folds open less (abduction 0.22, as the tense stops') and close as the frication ends, so its vowel starts ~19 ms after it (Seoul 9-14). Its fold tension is 0.6, not the stops' 0.9: at 0.9 a male voice starting that soon is aperiodic for 30-40 ms.

Sung, the stops aspirate far less: Sing scales a front end's `vot` by `sing::SUNG_VOT` (0.35), so lax and aspirated stops land at 22-27 and 32 ms, near pop vocalists' ㅂ 27.7 and ㅍ 32.7 (이철희·한경훈 2020); the tense ones stay at 19-21 against 10.8. `chimera diagnose korean` measures all of these.

In speech the vowel after a tense or aspirated stop starts about 5 semitones above one after a lax stop (sung notes keep their melody). A short list of compounds and Sino-Korean words (솜이불 → [솜니불], 갈등 → [갈뜽]) is read from a respelling in `korean.rs`, since their ㄴ insertion and tensing need the word's parts.

Between voiced sounds a lax stop is voiced (아기 [agi]). ㄲ ㄸ ㅃ ㅉ ㅆ are `fortis` (IPA `k͈`): the folds stiffen (tension 0.9) from the middle of the closure to 80 ms past the release, so the vowel starts pressed, its spectrum flatter (H1-A3 down 7 dB female, 17 dB male, 40-60 ms after the release) and the same as a lax stop's from about 150 ms on. A Korean speaker's ear is still wanted.

### Old Hangul

Old Hangul is written in conjoining jamo (ᄒᆞᆫ is ᄒ ᆞ ᆫ), and the Korean front end reads it: a run of
initials, medials and finals is one block, and so one note. Besides the modern letters it reads ㆍ,
ㅿ [z], ㆁ [ŋ, as an initial too], ㆆ [ʔ], ㅸ [β], ㆎ (ㆍ then [j]), ᆢ ([j] then ㆍ), the initial
clusters ᄞ ᄠ ᄡ ᄢ ᄣ ᄧ ᄩ ᄭ ᄯ ᄲ ᄶ, the final ᇙ (said ㄹ; its ㆆ tenses the next consonant in
the word), ᅇ (read silent: its value is argued over), and the tone dots 〮 〯. A vowel right
after a vowel starts a syllable of its own. Old texts in the Hanyang private-use
encoding must be converted to jamo first.

The ㅂ series reads as clusters (ᄠᅳᆮ [ptɯt], ᄢᅢ [pskɛ]). For the ㅅ series, Heo Ung (1953)
argued for clusters and Lee Ki-Moon (1955) argued for fortis consonants. A later study
distinguishes individual forms by spelling history. The cluster reading ([sk]) and
`Options::siot_tense` alternative (ᄭᅮᆷ [k͈um]) illustrate the hypotheses.
[Research history](https://www.korean.go.kr/nkview/nklife/1997_4/1997_0404.pdf).
[Re-examination](https://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART003015247).

The default ㆍ is [ʌ], following the chart Ko cites from Lee Ki-Moon (1972, p. 137).
[Ko, §3.2.1.1](https://eggschool.org/wp-content/uploads/2021/07/Ko-2012_Thesis_published_2018.pdf).
The [ɐ] target is exploratory. [ɔ] is a comparison with modern Jeju.
`chimera diagnose middle-korean [outdir]` reports F1/F2 for the three targets and, with an output
directory, renders both ㅅ-series readings of ᄭᅮᆷ and ᄯᆞᄅᆞᆷ.

In connected Middle Korean speech, written ᇙ can tense an eligible onset of the immediately
following Korean word in the same phrase (호ᇙ〮 배〮). Phrase punctuation resets the link.
[National Institute of Korean Language](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=73&pageIndex=1&qna_seq=324332).

### Readings: dialects and the 15th century

`korean::Options.dialect` picks how Hangul reads: the Korean reading menu under the text on the
Speak and Sing screens (standard, the regional dialects, Middle Korean), `chimera render text
<out> "…" male <reading>`, or the speech path's `EnglishG2P.korean`.

Beside it, the Old Hangul keyboard (`src/ui/components/KoreanInput.tsx`) types what an ordinary
keyboard can't: initials, vowels and finals as conjoining jamo (the archaic letters and the
clusters included), and the tone dots and the accent mark. It moves from initial to vowel to final
as you type, and types at the caret without raising a phone's own keyboard. Speak's examples
include the preface of Hunminjeongeum in its 15th-century Korean (the 언해본's; the 해례본's own
preface is Classical Chinese, which the voice can't read), written in Hangul with the 동국정운 readings
for Hanja and the [transcribed tone dots](https://atlas.mkhz.kr/archive/hunminjeongeum-eonhae/hmje_01_seomun). The dots were checked against the [facsimile on slide 4](https://contents.kocw.or.kr/document/lec/2012/ChungNam/HongSungShim/09.pdf). Picking the example switches to Middle Korean. English's allophones
(flapping, dark /l/) apply to English only.

**seoul** (the menu's "Standard Korean" and the app default, and what an omitted API/CLI reading selects) is the
standard pronunciation above alone: ㅐ and ㅔ kept apart, no accentual-phrase pitch.

The six regional readings cover selected pronunciation features. They apply to the spelling
entered and depend on the user for regional vocabulary, endings, and lexical tone marks.

- **central**: Seoul-based, with a modern merger of ㅐ and ㅔ.
  The accentual phrase starts low or high according to its onset and normally ends high.
  [K-ToBI](https://sunahjun.humspace.ucla.edu/ktobi/K-tobi.html).
  [Seoul vowel merger](https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART002914117).
- **southwest**: ㅢ reads [ɯ], as in 의사. The Chonnam contour starts low or high,
  peaks near the start of the accentual phrase, and ends low.
  [Southwestern dialect](https://encykorea.aks.ac.kr/Article/E0027670).
  [Jun 1998](https://linguistics.ucla.edu/people/jun/papers%20in%20pdf/Jun-Phonology1998.pdf).
- **southeast** (alias **gyeongsang**): ㅓ and ㅡ merge as [ə] and ㅆ reads as ㅅ.
  An acute marks a high syllable (가́지). Pitch rises within the marked vowel, stays high
  on adjacent marked syllables, then targets a low two syllables after the last marked
  vowel if the word is long enough. Focus and age-specific timing are still unmodelled.
  The syllable after the last high steps down 3.29 semitones (not to low), and an accent after
  low syllables rises 6.91 from low: older South Kyungsang word ranges HL 3.29, HH 3.59, LH 6.91
  (Lee & Jongman 2015); rendered 3.2-3.4, 3.6-3.8 and 7.2.
  Lax and aspirated stops keep apart by VOT (rendered 33-38 against 104 ms; South Gyeongsang
  men 38 and 104, 이현정·Jongman 2012), and a phrase's first ㄴ ㅁ keeps its nasality (Park, Jang,
  Kim & Cho 2025).
  [Jun et al. 2006](https://linguistics.ucla.edu/people/jun/papers%20in%20pdf/Jun_Kim_Lee_Jun-J%20of%20EastAsianLing-2006.pdf).
- **jeju**: ㆍ is [ɔ] following the recorded-vowel analysis of
  [Kim 2005](https://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART000965998), and ᆢ is [jɔ].
  Medial accentual phrases use a low ending, frequently observed in conversational Jeju.
  [Kim and Baek 2022](https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART002829883).
- **northwest**: suppresses ㄷ/ㅌ palatalization before 이 (같이); ㅈ-series affricates use an
  alveolar place. [Northwestern dialect](https://encykorea.aks.ac.kr/Article/E0027782).
  With the northeast, lax and aspirated stops keep apart by VOT (rendered 19-22 against 80 ms;
  North Korean speakers 21.9 and 81.1, Jung, Oh & Han 2025). In the northeast the stops barely
  move the vowel's pitch (`Segment::onset_f0`: lax -0.7, aspirated +0.7, tense -0.35 semitones;
  young Yanbian speakers, 오미라·양휘 2013), where Seoul's differ by about 5.
- **northeast**: takes northern Hamgyong alveolar affricates and ㅢ as [i]. An acute or one
  dot marks high (가〮), and two dots mark a long rise (가〯), following the three-way inventories
  described for Gilju, Seongjin and Dancheon. Tone marks are input annotations.
  [Northeastern dialect](https://encykorea.aks.ac.kr/Article/E0016526).
- **middle**: 15th-century vowels ㅓ [ə], ㅡ [ɨ], ㆍ by its hypothesis, and ㅐ ㅔ ㅚ ㅟ ㅢ ㆎ ending in
  [j]. Final ㅅ stays [s]. Nothing palatalises (같이 [kati]; that came in the 17th-18th centuries)
  and ㅅ ㅈ ㅊ are dental before any vowel ([si], [tsi]), 이기문's 치음; ㅅ is not aspirated. No dot gives low (平聲), one dot gives high (去聲), and two give a rise
  inside a lengthened vowel (上聲). The low and high targets are 90% and 115% of the selected
  speaking pitch, with the transition across the middle 40% of the vowel. These are synthesis
  settings. The planner gives explicit lexical tones priority over its default phrase contour.

Seoul, Chonnam and Jeju group adjacent whole Korean words until the group reaches three
syllables, stopping at punctuation or a language change. This is a heuristic for prosodic
phrasing. A question mark triggers a sentence-final rise and other sentence endings a fall.

#### Comparison with recorded-speaker measurements

- Middle Korean: in the 110 Hz male voice, the 250 ms vowel of 말〯 reached 99, 104 and 122 Hz
  at 25%, 50% and 75% of its duration (engine telemetry). 말 without a dot stayed near 99 Hz,
  and 말〮 near 126 Hz.
- Southeastern: the accent uses pitch targets of 90% and 121% of baseline (about 5.1
  semitones), with the rise planned between 15% and 75% of the vowel. This is a synthesis
  setting informed by the penultimate contrast for speaker JH (about 5.2 semitones in
  subject position in Jun et al. 2006, table 1). In an engine spot check on 어무́이,
  the preceding vowel held near 100 Hz and the marked vowel rose from 99 to 121 Hz
  (about 3.3 semitones), reflecting the short vowel and smoothing between targets.
- Jeju: a 30 ms window at the first vowel's midpoint in a male synthesis of ᄆᆞᆯ gave
  F1 554–621 Hz and F2 974–1062 Hz (LPC orders 12, 14 and 16 at 12 kHz). The older male
  speaker in [Kim et al. 2018](https://www.eksss.org/archive/view_article?pid=pss-10-1-57)
  averaged 549 and 954 Hz across the study items. The estimates depend on the measurement
  method, and this comparison uses the published averages.
- A direct spot check used the National Institute of Korean Language's recordings labeled
  톨/고지 (the slash denotes ㆍ), Hangyeong and Seogwipo, and the synthesis of ᄐᆞᆯ고지.
  In hand-selected first-vowel windows, median F1/F2 were approximately 500/1380 Hz
  for Hangyeong (0.475–0.500 s) and 650/1200 Hz for Seogwipo (0.320–0.350 s).
  The male and female synthesis presets gave about 590/1150 and 670/1400 Hz respectively
  (0.265–0.285 s). All values use 30 ms, order-14 LPC at 12 kHz. The clips vary with
  the measurement window, and the presets are not matched speakers.
  [Regional word recordings](https://www.korean.go.kr/front/page/pageView.do?mn_id=54&page_id=P000053).

Sung lyrics read in the same setting; in a song the melody and the notes set pitch and length, so
accents and tone dots do nothing there.

Not done, and why: a morphological analyser (ㄴ insertion and 사잇소리 need a word's parts; it would
bring a dictionary and a dependency the engine doesn't have, so a short respelling list stands in);
a Gyeongsang accent lexicon (which syllable is high is survey data this repository doesn't have);
and Hanyang private-use conversion (a table of thousands of code points to get exactly right).

## The one format

Every front end writes the same thing, one sung syllable per sounding note:

```json
{ "text": "풀", "onset": [{"ipa": "p", "aspirated": true, "vot": 0.07}], "nucleus": {"ipa": "u"},
  "coda": [{"ipa": "l"}], "wordEnd": true, "phraseEnd": false }
```

- `onset` / `coda`: consonants and glides from the inventory (`phonetics/inventory.rs`).
- A stop or affricate may say `"aspirated": true|false` (unsaid: English's rule; `ʰ` and `˭` in
  IPA text) and `"vot"`, its aspiration time in seconds relative to the 50 ms default (unsaid:
  English's 50 ms). Each millisecond more or less than 50 moves the voice's return by a
  millisecond; the measured voice onset also depends on the place and the voice (the Korean table
  above: 45 asked gives 53–69 ms, 70 gives about 80).
- `nucleus`: one vowel, or a diphthong the planner knows.
- A melisma note is `"melisma": true` with the same nucleus; its coda moves to the run's last note.
- `text` is what the lyrics panel and the piano roll show.
- `wordEnd` and `phraseEnd` may be left out (false).

Any front end's output can be frozen to JSON, edited and sung with `"syllables"` instead of
`"lyrics"`, in `render notes` and duet melody files. Use it to fix a reading
by hand, or to sing a language that has no front end yet.

## Adding a language

1. **Map its sounds onto the inventory.** Add missing ones between their neighbours, as
   `ɕ ʑ tɕ dʑ tʂ dʐ ɥ` were, and let `chimera calibrate consonants` set a fricative's constriction:
   it appends new entries and leaves existing ones alone.
2. **Write the front end**: implement `FrontEnd` (which characters start and continue its words,
   and word → syllables; `word_after` also gets the word before, for words that lean on each
   other, as a Russian preposition with no vowel does), then add it to the router's list in `lyrics_to_syllables`. A front end
   that needs data should load it like CMUdict: a gzipped file in `public/data/`, fetched only when
   the lyrics need it.
3. **Say the phonology** in the syllables (aspiration and its length). Build `Syllable` and
   `Segment` values with `..Default::default()` so later fields don't break you.
4. **Leave English unchanged.** Tuning the shared voice goes through `docs/hill-climb/README.md`.
5. **Run the scramble test**, below, and **have a native speaker listen.**

## The scramble test

`tools/asr/scramble.py` (set up once with `tools/asr/setup.sh`) measures how intelligible a front
end's singing is on sound alone. A recogniser that knows a lyric fills it in, so it also sings the
same melody with the lyric's units shuffled (syllables for Chinese and Korean, words for English),
leaving nothing to guess from.

- **Material:** public-domain texts: two Tang poems (静夜思, 春晓), the folk song 아리랑, Harvard
  sentences. `--text lang:file` sings your own lines; `tools/asr/probes/` has lines dense in one
  contrast each. Material dense in what a change touches is the only way to see it.
- **Rendering:** through CHIMERA's own front ends (`render notes` with lyrics), one note per
  syllable on one generated tune, a breath between lines, in the male and female voices.
  A language whose script no front end reads yet is skipped.
- **Judge:** SenseVoice-Small (sherpa-onnx), one sung line at a time. It's CTC, so it can't
  hallucinate a loop.
- **Score:** syllable errors (Chinese as toneless pinyin, Korean as Hangul blocks) or word errors
  (English), for the real lyric and the mean over the scrambles.
- **What goes wrong:** each line is aligned with what was heard and the errors split into a
  syllable's parts (Chinese: initial / final / tone; Korean: initial / vowel / final; English:
  phonemes, wrong vs dropped), with the commonest confusions. `--analyze <run>/scramble.json`
  re-reads a run.

On this branch (8 scrambles × 2 voices):

| Language (units) | Real lyric (male / female) | Scrambled (male / female) |
|---|---|---|
| Korean, 아리랑 (syllables) | 30.0 / 15.0% | 52.8 / 55.6% |
| English, Harvard (words) | 48.8 / 53.7% | 68.6 / 75.0% |

The gap between the real and scrambled lyric is what the recogniser fills in from knowing the
text; the scrambled number is the sound. Korean's commonest errors are ㄹ heard as ㄴ or ㄷ and a
coda ㄴ dropped; English's are sounds dropped (a third of vowels and consonants), AH heard as ER,
SH as F and D as T.

Treat about ±2 points as noise at these sizes (8 scrambles × 2 voices) and read the confusion
counts for the sounds a change targets, not just the total. One recogniser and one tune can't stand
in for a native listener; the test says where to look and whether a change moved what it aimed at.

## Not yet

- **Latin-script languages.** Routing by script can't tell Turkish or German from English, so
  Latin text is English. The second Latin-script front end brings an explicit language choice,
  with English the default (#40).
- **Tone and pitch accent** need pitch contours (#32 step 4).
