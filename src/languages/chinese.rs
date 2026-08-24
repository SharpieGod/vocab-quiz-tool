use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use strsim::normalized_levenshtein;
use vocab_quiz_tool::{Language, Quizzable};

use super::english::English;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct ChineseWord {
    hanzi: String,
    definitions: Vec<English>,
    pinyin: Chinese,
}

impl Display for ChineseWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.pinyin, self.hanzi)
    }
}

impl Quizzable<English, Chinese> for ChineseWord {
    fn unique_key(&self) -> String {
        format!(
            "{} {}",
            self,
            self.definitions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }

    fn get_definitions(&self) -> Vec<&English> {
        self.definitions.iter().collect()
    }

    fn get_word(&self) -> &Chinese {
        &self.pinyin
    }

    fn definitions_match(guess: &English, definition: &English) -> bool {
        guess.0 == definition.0
            || definition
                .0
                .split(&[';', ',', '(', '/'][..])
                .map(|part| part.trim().trim_end_matches(')').to_lowercase())
                .any(|part| normalized_levenshtein(&guess.0.trim().to_lowercase(), &part) >= 0.85)
    }

    fn words_match(a: &Chinese, b: &Chinese) -> bool {
        a.0.iter()
            .zip(b.0.iter())
            .map(|(a, b)| {
                let mut s = 0.0;

                if a.text == b.text {
                    s += 0.7;
                }

                if a.tone == b.tone {
                    s += 0.3;
                }

                s
            })
            .sum::<f32>()
            / a.0.len() as f32
            >= 0.95
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Chinese(Vec<ChineseCharacter>);
impl Language for Chinese {}

impl FromStr for Chinese {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(Self(
            s.split_whitespace()
                .map(|p| p.parse::<ChineseCharacter>())
                .collect::<Result<Vec<ChineseCharacter>, _>>()?,
        ))
    }
}

impl Display for Chinese {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Debug)]
struct ChineseCharacter {
    tone: ChineseTone,
    text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sandhi_from: Option<ChineseTone>,
}

impl ChineseCharacter {
    fn accented_vowel(c: char, tone: ChineseTone) -> char {
        match (c, tone) {
            ('a', ChineseTone::High) => 'ā',
            ('a', ChineseTone::Rising) => 'á',
            ('a', ChineseTone::Dropping) => 'ǎ',
            ('a', ChineseTone::Falling) => 'à',
            ('a', ChineseTone::Neutral) => 'a',

            ('e', ChineseTone::High) => 'ē',
            ('e', ChineseTone::Rising) => 'é',
            ('e', ChineseTone::Dropping) => 'ě',
            ('e', ChineseTone::Falling) => 'è',
            ('e', ChineseTone::Neutral) => 'e',

            ('i', ChineseTone::High) => 'ī',
            ('i', ChineseTone::Rising) => 'í',
            ('i', ChineseTone::Dropping) => 'ǐ',
            ('i', ChineseTone::Falling) => 'ì',
            ('i', ChineseTone::Neutral) => 'i',

            ('o', ChineseTone::High) => 'ō',
            ('o', ChineseTone::Rising) => 'ó',
            ('o', ChineseTone::Dropping) => 'ǒ',
            ('o', ChineseTone::Falling) => 'ò',
            ('o', ChineseTone::Neutral) => 'o',

            ('u', ChineseTone::High) => 'ū',
            ('u', ChineseTone::Rising) => 'ú',
            ('u', ChineseTone::Dropping) => 'ǔ',
            ('u', ChineseTone::Falling) => 'ù',
            ('u', ChineseTone::Neutral) => 'u',

            ('ü' | 'v', ChineseTone::High) => 'ǖ',
            ('ü' | 'v', ChineseTone::Rising) => 'ǘ',
            ('ü' | 'v', ChineseTone::Dropping) => 'ǚ',
            ('ü' | 'v', ChineseTone::Falling) => 'ǜ',
            ('ü' | 'v', ChineseTone::Neutral) => 'ü',

            _ => c,
        }
    }
}

impl FromStr for ChineseCharacter {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tone = {
            let Some(last_char) = s.chars().last() else {
                return Err(());
            };

            match last_char {
                '1' => ChineseTone::High,
                '2' => ChineseTone::Rising,
                '3' => ChineseTone::Dropping,
                '4' => ChineseTone::Falling,
                _ => ChineseTone::Neutral,
            }
        };

        let take = if tone == ChineseTone::Neutral {
            s.len()
        } else {
            s.len() - 1
        };

        let text = s.chars().take(take).collect::<String>();

        Ok(Self {
            tone,
            text: text.to_lowercase(),
            sandhi_from: None,
        })
    }
}

impl Display for ChineseCharacter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(sandhi) = self.sandhi_from {
            write!(f, "{}{sandhi}->{}", self.text, self.tone)?;
        } else {
            write!(f, "{}{}", self.text, self.tone)?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug)]
enum ChineseTone {
    High,
    Rising,
    Dropping,
    Falling,
    Neutral,
}

impl Display for ChineseTone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::High => "1",
                Self::Rising => "2",
                Self::Dropping => "3",
                Self::Falling => "4",
                Self::Neutral => "0",
            }
        )
    }
}
