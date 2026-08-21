use std::{
    fmt::{Debug, Display},
    hash::Hash,
    str::FromStr,
};

use serde::{Deserialize, Serialize, Serializer, de::DeserializeOwned};
use vocab_quiz_tool::{Dictionary, Language, Quizzable, VocabError};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
struct English(String);
impl Language for English {}

impl FromStr for English {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl Display for English {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct French(String);

impl Language for French {}

impl FromStr for French {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl Display for French {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct BasicWord {
    word: French,
    definitions: Vec<English>,
}

impl Display for BasicWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {}",
            self.word,
            self.definitions
                .iter()
                .map(|d| d.0.clone())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Debug for BasicWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", format!("{}", self))
    }
}

impl Quizzable<English, French> for BasicWord {
    fn unique_key(&self) -> String {
        self.word.to_string()
    }

    fn get_definitions(&self) -> Vec<&English> {
        self.definitions.iter().collect()
    }

    fn get_word(&self) -> &French {
        &self.word
    }

    fn definitions_match(a: &English, b: &English) -> bool {
        a.0.trim().to_lowercase() == b.0.trim().to_lowercase()
    }

    fn words_match(a: &French, b: &French) -> bool {
        a.0 == b.0
    }
}

fn main() -> Result<(), VocabError> {
    let mut d = Dictionary::<BasicWord, English, French>::new("dictionaries/basic.json")?;

    println!("Here's the data: ");
    println!("{:#?}", d.data);

    d.run_drill(true)?;

    Ok(())
}
