use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use vocab_quiz_tool::Language;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct English(pub String);

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

impl Language for English {}
