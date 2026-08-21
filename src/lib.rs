use std::{
    collections::VecDeque,
    fmt::{Debug, Display},
    fs,
    io::{self, Write, stdout},
    path::Path,
    str::FromStr,
};

use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};

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
    fn definitions_match(a: &N, b: &N) -> bool;
    fn words_match(a: &F, b: &F) -> bool;
    fn matches_word(&self, cmp: &F) -> bool {
        Self::words_match(self.get_word(), cmp)
    }
    fn matches_definition(&self, cmp: &N) -> bool {
        self.get_definitions()
            .iter()
            .any(|f| Self::definitions_match(f, cmp))
    }
    fn with_definitions(&self, extra_indent: usize) -> String {
        let joined = self.get_word().to_string();
        let indent = " ".repeat(joined.chars().count().saturating_add(extra_indent));

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

    pub fn run_drill(&mut self, foreign_to_native: bool) -> Result<(), VocabError> {
        let mut rng = rand::rng();

        let mut queue: VecDeque<&(Q, f32)> = {
            let mut q: Vec<&(Q, f32)> = self.data.iter().collect();
            q.shuffle(&mut rng);

            q.iter().copied().collect()
        };

        while let Some(n) = queue.pop_back() {
            clear_console()?;
            let (word, mastery) = n;

            let prompt = if foreign_to_native {
                word.get_word().to_string()
            } else {
                word.get_definitions()
                    .choose(&mut rng)
                    .map_or(String::new(), |d| d.to_string())
            };

            print!("{} - ", prompt);
            io::stdout().flush()?;

            let guess = input()?;

            let matches = if foreign_to_native {
                // see if what you typed matches any definition of `word`

                let native = guess
                    .parse::<N>()
                    .map_err(|_| VocabError::ParseError(guess.clone()))?;

                word.matches_definition(&native)
            } else {
                // see if what you typed has the definition of `word`

                let foreign = guess
                    .parse::<F>()
                    .map_err(|_| VocabError::ParseError(guess.clone()))?;

                self.data.iter().any(|(w, _)| {
                    w.matches_word(&foreign)
                        && word
                            .get_definitions()
                            .iter()
                            .any(|d| w.matches_definition(d))
                })
            };

            clear_console()?;

            println!("{}", word.with_definitions(0));
            println!("{}", if matches { "correct" } else { "incorrect" });
            input()?;
            queue.push_front(n);
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
