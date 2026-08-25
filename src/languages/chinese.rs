use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use strsim::{normalized_damerau_levenshtein, normalized_levenshtein};
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

                if a.text == b.text
                    || normalized_damerau_levenshtein(&a.text, &b.text) >= 0.75
                {
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

    fn play_audio(&self) {
        play_chinese_audio(&self.hanzi);
    }
}

/// Speaks `text` in the background: a natural neural voice if reachable, otherwise the
/// local Windows SAPI voice as an offline fallback.
fn play_chinese_audio(text: &str) {
    let text = text.to_string();

    let _ = std::thread::Builder::new().spawn(move || {
        if speak_edge_neural(&text).is_none() {
            speak_chinese_sapi(&text);
        }
    });
}

const AUDIO_CACHE_DIR: &str = "audio/chinese";

/// Path of the cached mp3 for `hanzi`, e.g. `audio/chinese/休息.mp3`.
fn cached_audio_path(hanzi: &str) -> PathBuf {
    let safe_name: String = hanzi
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();

    Path::new(AUDIO_CACHE_DIR).join(format!("{safe_name}.mp3"))
}

/// Tries to speak `text` with a neural Chinese voice via the (free, unofficial) Microsoft
/// Edge "Read Aloud" service, using a local mp3 cache to skip the network on repeat plays.
/// Returns `None` on any failure (e.g. no network and nothing cached yet) so the caller
/// can fall back to a local voice; blocks the calling thread until playback finishes.
fn speak_edge_neural(text: &str) -> Option<()> {
    let path = cached_audio_path(text);

    let audio_bytes = if let Ok(bytes) = fs::read(&path) {
        bytes
    } else {
        let bytes = fetch_neural_audio(text)?;

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, &bytes);

        bytes
    };

    play_mp3_bytes(&audio_bytes)
}

/// Synthesizes `text` via the Edge "Read Aloud" service and returns the raw mp3 bytes.
fn fetch_neural_audio(text: &str) -> Option<Vec<u8>> {
    use msedge_tts::{
        tts::{SpeechConfig, client::connect},
        voice::get_voices_list,
    };

    let voices = get_voices_list().ok()?;

    let voice = voices
        .iter()
        .find(|v| v.locale.as_deref() == Some("zh-CN") && v.name.contains("Xiaoxiao"))
        .or_else(|| voices.iter().find(|v| v.locale.as_deref() == Some("zh-CN")))?;

    let config = SpeechConfig::from(voice);
    let mut tts = connect().ok()?;
    let audio = tts.synthesize(text, &config).ok()?;

    Some(audio.audio_bytes)
}

fn play_mp3_bytes(bytes: &[u8]) -> Option<()> {
    let mut handle = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
    handle.log_on_drop(false);
    let player = rodio::Player::connect_new(&handle.mixer());
    let decoder = rodio::decoder::Decoder::new(std::io::Cursor::new(bytes.to_vec())).ok()?;

    player.append(decoder);
    player.sleep_until_end();

    Some(())
}

/// Speaks `text` using a zh-CN voice via Windows Speech (SAPI), in the background.
///
/// Shells out to PowerShell rather than talking to SAPI directly, since the OS-installed
/// desktop voices (e.g. "Microsoft Huihui Desktop") are exposed there without extra deps.
fn speak_chinese_sapi(text: &str) {
    static SCRIPT_COUNTER: AtomicU64 = AtomicU64::new(0);

    let escaped = text.replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Speech\n\
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer\n\
         $v = $s.GetInstalledVoices() | Where-Object {{ $_.VoiceInfo.Culture.Name -eq 'zh-CN' }} | Select-Object -First 1\n\
         if ($v) {{ $s.SelectVoice($v.VoiceInfo.Name) }}\n\
         $s.Speak('{escaped}')\n\
         Remove-Item -LiteralPath $MyInvocation.MyCommand.Path -Force\n"
    );

    let n = SCRIPT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "vocab_quiz_tool_speak_{}_{n}.ps1",
        std::process::id()
    ));

    // UTF-8 BOM so PowerShell reads the script (and the embedded hanzi) as UTF-8.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(script.as_bytes());

    if fs::write(&path, bytes).is_err() {
        return;
    }

    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
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
