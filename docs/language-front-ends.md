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

Closure lasts longest for tense stops (123 ms), then aspirated (84), then lax (48), the 2026 notes' VCV means; ㅎ between vowels is brief (it is mostly dropped in real speech).

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

The clusters: the ㅂ series reads as clusters (ᄠᅳᆮ [ptɯt], ᄢᅢ [pskɛ]); the ㅅ series is argued over,
so it reads as clusters ([sk]) by default or, with `Options::siot_tense`, as tense consonants
(ᄭᅮᆷ [k͈um]). A stop after a cluster's consonants doesn't aspirate.

ㆍ's value is argued over too, so it has hypotheses (`korean::ARAE_A`): [ʌ] (이기문 1972; the
default), [ɐ] and [ɔ] (sources not recorded yet). `chimera diagnose middle-korean [outdir]` says
ᄒᆞᆫ ᄆᆞᅀᆞᆷ ᄉᆞᄅᆞᆷ ᄃᆞᆯ under each in the 15th-century reading (below), with the vowel's F1/F2 beside modern
ㅏ ㅓ ㅗ and WAVs to compare (male: [ʌ] 584/1223 Hz, [ɐ] 605/1382, [ɔ] 562/977). A render shows what
a hypothesis sounds like in this voice; it is not evidence for it.

### Readings: dialects and the 15th century

`korean::Options.dialect` picks how Hangul reads: the Korean reading menu under the text on the
Speak and Sing screens (standard, the regional dialects, Middle Korean), `chimera render text
<out> "…" male <reading>`, or the speech path's `EnglishG2P.korean`.

Beside it, the Old Hangul keyboard (`src/ui/components/KoreanInput.tsx`) types what an ordinary
keyboard can't: initials, vowels and finals as conjoining jamo (the archaic letters and the
clusters included), and the tone dots and the accent mark. It moves from initial to vowel to final
as you type, and types at the caret without raising a phone's own keyboard. Speak's examples
include the preface of Hunminjeongeum in its 15th-century Korean (the 언해본's; the 해례본's own
preface is Classical Chinese, which the voice can't read), with its Chinese characters beside
their 동국정운 readings and the [transcribed tone dots](https://atlas.mkhz.kr/archive/hunminjeongeum-eonhae/hmje_01_seomun). The dots were checked against the [facsimile on slide 4](https://contents.kocw.or.kr/document/lec/2012/ChungNam/HongSungShim/09.pdf). Picking the example switches to Middle Korean. English's allophones
(flapping, dark /l/) apply to English only.

The six regional readings cover selected pronunciation features. They apply to the spelling
entered and depend on the user for regional vocabulary, endings, and lexical tone marks.

- **central** (the app default): Seoul-based, with a modern merger of ㅐ and ㅔ.
  [Seoul vowel merger](https://www.kci.go.kr/kciportal/ci/sereArticleSearch/ciSereArtiView.kci?sereArticleSearchBean.artiId=ART002914117).
  Omitted API/CLI readings and the legacy **seoul** identifier still select the original standard reading.
- **southwest**: ㅢ reads [ɯ], as in 의사. [Southwestern dialect](https://encykorea.aks.ac.kr/Article/E0027670).
- **southeast** (alias **gyeongsang**): ㅓ and ㅡ merge as [ə] and ㅆ reads as ㅅ.
  An acute marks a high syllable (가́지). The speech planner approximates the accent with
  its existing pitch and length targets. Word-initial low boundaries, the following low,
  focus and age-specific timing are still unmodelled.
  [Jun et al. 2006](https://linguistics.ucla.edu/people/jun/papers%20in%20pdf/Jun_Kim_Lee_Jun-J%20of%20EastAsianLing-2006.pdf).
- **jeju**: ㆍ is [ɔ] following the recorded-vowel analysis of
  [Kim 2005](https://www.kci.go.kr/kciportal/landing/article.kci?arti_id=ART000965998), and ᆢ is [jɔ].
- **northwest**: suppresses ㄷ/ㅌ palatalization before 이 (같이); ㅈ-series affricates use an
  alveolar place. [Northwestern dialect](https://encykorea.aks.ac.kr/Article/E0027782).
- **northeast**: takes northern Hamgyong alveolar affricates and ㅢ as [i]. An acute or one
  dot marks high (가〮), and two dots mark a long rise (가〯), following the three-way inventories
  described for Gilju, Seongjin and Dancheon. Tone marks are input annotations.
  [Northeastern dialect](https://encykorea.aks.ac.kr/Article/E0016526).
- **middle**: 15th-century vowels ㅓ [ə], ㅡ [ɨ], ㆍ by its hypothesis, and ㅐ ㅔ ㅚ ㅟ ㅢ ㆎ ending in
  [j]. Final ㅅ stays [s]. No dot gives low (平聲), one dot gives high (去聲), and two give a rise
  inside a lengthened vowel (上聲). The low and high targets are 90% and 115% of the selected
  speaking pitch, with the transition across the middle 40% of the vowel. These are synthesis
  settings. The planner gives explicit lexical tones priority over its default phrase contour.

#### Comparison with recorded-speaker measurements

- Middle Korean: in the 110 Hz male voice, the 250 ms vowel of 말〯 reached 99, 104 and 122 Hz
  at 25%, 50% and 75% of its duration (engine telemetry). 말 without a dot stayed near 99 Hz,
  and 말〮 near 126 Hz.
- Southeastern: the synthesized 어무́이 contained a peak near 129 Hz in the marked second
  vowel, up from roughly 108 Hz in the first vowel (about 3 semitones). Jun et al. (2006),
  table 1, report a larger penultimate contrast for speaker JH (188 to 254 Hz in subject
  position, about 5.2 semitones). The peak falls in the expected vowel, and the current model
  has a smaller excursion in this spot check. The study also reports a later post-accent low
  beyond the model's simple phrase fall.
- Jeju: a 30 ms window at the first vowel's midpoint in a male synthesis of ᄆᆞᆯ gave
  F1 554–621 Hz and F2 974–1062 Hz (LPC orders 12, 14 and 16 at 12 kHz). The older male
  speaker in [Kim et al. 2018](https://www.eksss.org/archive/view_article?pid=pss-10-1-57)
  averaged 549 and 954 Hz across the study items. The estimates depend on the measurement
  method, and this comparison uses the published averages.

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
