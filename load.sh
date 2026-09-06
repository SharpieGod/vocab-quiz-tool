#!/usr/bin/env bash
# Converts any new Chinese lesson .docx files to .txt, then opens a Claude
# session with instructions to extract vocabulary from the not-yet-processed
# transcripts into dictionaries/chinese.json.
set -euo pipefail
cd "$(dirname "$0")"

if [ -x ".venv/bin/python3" ]; then
    PYTHON=".venv/bin/python3"
else
    PYTHON="python3"
fi

"$PYTHON" helper.py

PROMPT=$(cat <<'EOF'
Look in documents/chinese/txt/ for .txt files that do NOT have a "DONE_"
prefix. These are new lesson transcripts (converted from .docx by helper.py)
that still need their vocabulary extracted into dictionaries/chinese.json.
Files already prefixed "DONE_" have already been processed - ignore them
except as reference for formatting/style.

For each not-done file:

1. Read it and identify the new vocabulary headwords being taught (lines
   like "word - definition" or "word(-definition)"). Skip pure example
   sentences that only combine words already in the dictionary, and skip
   homework/instruction text - those aren't vocabulary.

2. For each headword, work out:
   - The correct hanzi. The files only contain pinyin with numeric tone
     marks (e.g. "kuai4", "xiang1 jiao1"); you must supply the hanzi
     yourself based on the meaning/context given.
   - The English definition(s), taken from the note (clean up phrasing but
     don't invent meanings that aren't implied by the note).

3. Before adding anything, check dictionaries/chinese.json for a
   duplicate: same hanzi AND same pinyin. If it already exists:
   - If it's the exact same word, skip it.
   - If the same hanzi+pinyin exists but with a NEW definition not already
     listed (e.g. a character has two senses), append the new definition
     string to that entry's existing "definitions" array instead of
     creating a duplicate entry.

4. dictionaries/chinese.json is a JSON array of [ChineseWord, masteryFloat]
   pairs. ChineseWord shape:
     {
       "hanzi": "<the word>",
       "definitions": ["<def1>", "<def2>", ...],
       "pinyin": [
         { "tone": "<ToneName>", "text": "<syllable, no tone number>",
           "sandhi_from": "<ToneName>"  // optional, only when tone sandhi applies
         },
         ...
       ]
     }
   One entry in "pinyin" per syllable/character, in order.

   Tone name mapping (numeric tone -> ToneName):
     1 -> "High"      (e.g. ma1 -> High)
     2 -> "Rising"    (e.g. ma2 -> Rising)
     3 -> "Dropping"  (e.g. ma3 -> Dropping)
     4 -> "Falling"   (e.g. ma4 -> Falling)
     neutral (no number, or 5) -> "Neutral"

   ü is written as "nü" in the text field (source notes often type it as
   "nu:" or "v", e.g. "nu:3" -> Dropping tone, text "nü").

   New words you add should get mastery 0.0 (the baseline for unpracticed
   words - look at existing 0.0 entries for reference).

5. Tone sandhi rules - set "sandhi_from" on the syllable whose *written*
   tone differs from its dictionary/citation tone, with sandhi_from set to
   that original citation tone. Look at existing entries in
   dictionaries/chinese.json for real examples of each pattern before
   applying it:
   - Third-tone sandhi: a Dropping (3rd tone) syllable immediately followed
     by another Dropping syllable is written as Rising with
     sandhi_from: "Dropping" (e.g. 你好, 水果, 哪里, 很好, 老板).
   - 一 (yi, citation tone High): when used as the number "one" acting as a
     quantifier before another word, it shifts: before a Falling
     (4th-tone) syllable it becomes Rising/sandhi_from High (e.g. 一件,
     一岁, 一共, 一刻); before a High/Rising/Dropping syllable it becomes
     Falling/sandhi_from High (e.g. 一般, 一杯, 一些, 一年, 一起, 一千,
     一百, 一两). When 一 is used as a label/ordinal instead (calendar
     months, dates, sequence names like 星期一, 一月, 一号), it keeps its
     citation tone High with no sandhi.
   - 不 (bu, citation tone Falling): before another Falling syllable it
     becomes Rising/sandhi_from Falling (e.g. 不客气, 不对, 不太).
     Otherwise it stays Falling with no sandhi (e.g. 不好意思, where the
     next syllable is Dropping not Falling).
   - Reduplicated family-term syllables (哥哥, 姐姐, 妈妈, 爸爸, 爷爷,
     奶奶, 弟弟, 妹妹, 宝宝): the repeated second syllable is written
     Neutral with sandhi_from set to the first syllable's citation tone.
   - The noun suffix 子 (zi, citation tone Dropping, as in 儿子, 房子,
     本子, 椅子, 桌子, 孩子, 橙子, 杯子): written as Neutral with
     sandhi_from: "Dropping".
   - The noun suffix 字 (zi, citation tone Falling, as in 名字): written
     as Neutral with sandhi_from: "Falling" (different from the 子 suffix
     above - don't confuse the two).
   - Erhua "r" suffix (哪儿, 这儿, 那儿): written as its own syllable
     entry { "tone": "Neutral", "text": "r" } with no sandhi_from.

6. After editing dictionaries/chinese.json, validate it parses
   (`python3 -c "import json; json.load(open('dictionaries/chinese.json'))"`)
   and run `cargo build` to confirm the Rust project still reads it
   correctly.

7. Rename each transcript file you've fully processed by prefixing it with
   "DONE_" (e.g. `mv documents/chinese/txt/20260901.txt
   documents/chinese/txt/DONE_20260901.txt`) so it's not reprocessed next
   time.

If there are no files without the "DONE_" prefix, just say so - there's
nothing to do.
EOF
)

exec claude "$PROMPT"
