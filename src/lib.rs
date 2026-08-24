use std::{
    collections::VecDeque,
    fmt::{Debug, Display},
    fs,
    io::{self, Write, stdout},
    path::Path,
    str::FromStr,
};

use colorize::AnsiColor;
use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use crate::VocabError::BasicError;

pub enum VocabError {
    SerdeJson(serde_json::Error),
    IOError(io::Error),
    ParseError(String),
    BasicError(String),
}

impl Debug for VocabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerdeJson(arg0) => f.debug_tuple("SerdeJson").field(arg0).finish(),
            Self::IOError(arg0) => f.debug_tuple("IOError").field(arg0).finish(),
            Self::ParseError(arg0) => f.debug_tuple("ParseError").field(arg0).finish(),
            Self::BasicError(arg0) => f.debug_tuple("BasicError").field(arg0).finish(),
        }
    }
}

impl From<io::Error> for VocabError {
    fn from(value: io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<serde_json::Error> for VocabError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}

// word contains
pub enum Answer<N, F> {
    Native(N),
    Foreign(F),
}

pub trait Language: FromStr + Eq + Clone + Display {}

pub trait Quizzable<N, F>: Serialize + for<'a> Deserialize<'a> + Display + Clone
where
    N: Language,
    F: Language,
{
    fn unique_key(&self) -> String;
    fn get_definitions(&self) -> Vec<&N>;
    fn get_word(&self) -> &F;
    fn definitions_match(guess: &N, definition: &N) -> bool;
    fn words_match(a: &F, b: &F) -> bool;
    fn matches_word(&self, cmp: &F) -> bool {
        Self::words_match(self.get_word(), cmp)
    }
    fn matches_definition(&self, guess: &N) -> bool {
        self.get_definitions()
            .iter()
            .any(|f| Self::definitions_match(guess, f))
    }
    fn with_definitions(&self, extra_indent: usize) -> String {
        let joined = self.to_string();
        let indent = " ".repeat(joined.width().saturating_add(extra_indent));

        self.get_definitions()
            .iter()
            .enumerate()
            .map(|(i, def)| {
                if i == 0 {
                    format!("{joined} - {def}")
                } else {
                    format!("{indent} - {def}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Hadles full `Quizable` functionality which Q
pub struct Dictionary<Q, N, F> {
    pub data: Vec<(Q, f32)>,
    pub path: String,
    pub _marker: std::marker::PhantomData<(N, F)>,
}

impl<Q, N, F> Dictionary<Q, N, F>
where
    Q: Quizzable<N, F>,
    N: Language,
    F: Language,
{
    pub fn save(&self) -> Result<(), VocabError> {
        let p = Path::new(&self.path);

        Ok(fs::write(p, serde_json::to_string_pretty(&self.data)?)?)
    }

    pub fn new(p: &str) -> Result<Self, VocabError> {
        let pth = Path::new(p);

        let s = fs::read_to_string(pth)?;

        let data = serde_json::from_str(&s)?;

        Ok(Self {
            path: p.to_string(),
            data,
            _marker: std::marker::PhantomData,
        })
    }

    fn find_all_words_that_match(&self, guess: &N) -> Vec<&Q> {
        self.data
            .iter()
            .filter_map(|(w, _)| w.matches_definition(guess).then_some(w))
            .collect()
    }

    pub fn run_drill(&mut self, foreign_to_native: bool) -> Result<(), VocabError> {
        let mut completed = 0;
        let mut streak = 0;

        let mut rng = rand::rng();
        let mut queue: VecDeque<usize> = {
            let mut ordered = (0usize..self.data.len()).collect::<Vec<usize>>();
            ordered.shuffle(&mut rng);

            ordered.into_iter().collect()
        };

        let mut generate_prompt = |word: &Q| {
            if foreign_to_native {
                (word.to_string(), word.with_definitions(0))
            } else {
                let d = word
                    .get_definitions()
                    .choose(&mut rng)
                    .map_or(String::new(), ToString::to_string);

                (d.clone(), format!("{d} - {}", word.get_word()))
            }
        };
        let guess_matches =
            |guess: &str, word: &Q, data: &Vec<(Q, f32)>| -> Result<bool, VocabError> {
                Ok(if foreign_to_native {
                    // see if what you typed matches any definition of `word`

                    let native = guess
                        .parse::<N>()
                        .map_err(|_| VocabError::ParseError(guess.to_string()))?;

                    word.matches_definition(&native)
                } else {
                    // see if what you typed has the definition of `word`

                    let foreign = guess
                        .parse::<F>()
                        .map_err(|_| VocabError::ParseError(guess.to_string()))?;

                    data.iter().any(|(w, _)| {
                        w.matches_word(&foreign)
                            && word
                                .get_definitions()
                                .iter()
                                .any(|d| w.matches_definition(d))
                    })
                })
            };

        while let Some(index) = queue.pop_back() {
            let mut user_override = false;

            clear_console()?;
            let (word, mastery) = self
                .data
                .get(index)
                .ok_or_else(|| BasicError("Index Error".to_string()))?;

            let (prompt, correct_display) = generate_prompt(word);

            println!(
                "{completed} {}",
                if streak > 0 {
                    format!("streak {streak}")
                } else {
                    String::new()
                }
            );

            print!("{prompt} - ");
            io::stdout().flush()?;

            let guess = input()?;

            if guess == "q" {
                break;
            }

            let matches: bool = guess_matches(&guess, word, &self.data)?;

            loop {
                let got_right = matches || user_override;

                let delta_m = (if got_right {
                    mastery.mul_add(0.05, 5.0)
                } else {
                    -mastery.mul_add(0.1, 10.0)
                })
                .clamp(-*mastery, 100.0 - *mastery);

                let deck_size = queue.len().max(1);

                let insert_at: usize = if got_right {
                    // low mastery -> reappears in maybe 1/10th of the deck
                    // high mastery -> reappears near a full deck length out (or beyond, if you want)
                    let frac = (*mastery / 100.0).mul_add(1.4, 0.1); // ranges ~0.1 to ~1.5
                    ((deck_size as f32) * frac).round() as usize
                } else {
                    (deck_size / 15).max(5)
                };

                let insert_at = insert_at.min(queue.len());

                clear_console()?;
                println!(
                    "{} {}",
                    completed + 1,
                    if got_right {
                        format!("streak {}", streak + 1)
                    } else {
                        String::new()
                    }
                );

                println!("{correct_display}\n");
                println!(
                    "{} {:.2} ({:+.2})",
                    match (matches, user_override) {
                        (true, _) => "correct".green(),
                        (false, true) => "overriden".yellow(),
                        (false, false) => "incorrect".red(),
                    },
                    mastery + delta_m,
                    delta_m
                );

                if foreign_to_native && !got_right {
                    let words = self.find_all_words_that_match(
                        &guess
                            .parse()
                            .map_err(|_| VocabError::ParseError(guess.clone()))?,
                    );

                    if !words.is_empty() {
                        println!(
                            "{guess} - {}",
                            words
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                } else if !foreign_to_native && !got_right {
                }

                if input()?.as_str() == "o" {
                    user_override = true;
                    continue;
                }

                self.data
                    .get_mut(index)
                    .ok_or_else(|| BasicError("Index Error Write".to_string()))?
                    .1 += delta_m;

                self.save()?;
                queue.insert(insert_at, index);
                completed += 1;

                if got_right {
                    streak += 1;
                } else {
                    streak = 0;
                }

                break;
            }
        }

        Ok(())
    }
}

pub fn input() -> Result<String, VocabError> {
    let mut out = String::new();
    std::io::stdin().read_line(&mut out)?;

    Ok(out.trim().to_string())
}

fn clear_console() -> Result<(), VocabError> {
    // \x1B[3J clears the scrollback buffer
    // \x1B[2J clears the visible screen
    // \x1B[H moves the cursor to the top-left corner
    print!("\x1B[3J\x1B[2J\x1B[H");

    io::stdout().flush()?;

    Ok(())
}
