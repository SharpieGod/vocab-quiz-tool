mod languages;

use languages::chinese::{Chinese, ChineseWord};
use languages::english::English;
use vocab_quiz_tool::{Dictionary, Quizzable, VocabError};

fn main() -> Result<(), VocabError> {
    let mut d = Dictionary::<ChineseWord, English, Chinese>::new("dictionaries/chinese.json")?;

    d.show_menu()?;

    Ok(())
}
