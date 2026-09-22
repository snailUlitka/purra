use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::{TextEngine, TextEngineError, TextRule};

// Unicode 17.0 General_Category=Dash_Punctuation, excluding the normalized
// ASCII HYPHEN-MINUS. MINUS SIGN is included because it is commonly used as a
// typographic hyphen despite belonging to General_Category=Math_Symbol.
pub(crate) const AI_DASHES: &[char] = &[
    '\u{058a}',
    '\u{05be}',
    '\u{1400}',
    '\u{1806}',
    '\u{2010}',
    '\u{2011}',
    '\u{2012}',
    '\u{2013}',
    '\u{2014}',
    '\u{2015}',
    '\u{2212}',
    '\u{2e17}',
    '\u{2e1a}',
    '\u{2e3a}',
    '\u{2e3b}',
    '\u{2e40}',
    '\u{2e5d}',
    '\u{301c}',
    '\u{3030}',
    '\u{30a0}',
    '\u{fe31}',
    '\u{fe32}',
    '\u{fe58}',
    '\u{fe63}',
    '\u{ff0d}',
    '\u{10d6e}',
    '\u{10ead}',
];

// Unicode 17.0 General_Category=Space_Separator, excluding ASCII SPACE.
pub(crate) const AI_SPACES: &[char] = &[
    '\u{00a0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{202f}', '\u{205f}', '\u{3000}',
];

pub(crate) const AI_DOUBLE_QUOTES: &[char] = &['\u{00ab}', '\u{00bb}', '“', '”', '„', '‟'];
pub(crate) const AI_SINGLE_QUOTES: &[char] = &['‘', '’', '‚', '‛'];

const ASCII_EXPANSIONS: &[(char, &str)] = &[
    ('…', "..."),
    ('‥', ".."),
    ('ﬀ', "ff"),
    ('ﬁ', "fi"),
    ('ﬂ', "fl"),
    ('ﬃ', "ffi"),
    ('ﬄ', "ffl"),
    ('ﬅ', "st"),
    ('ﬆ', "st"),
    ('→', "->"),
    ('⟶', "->"),
    ('←', "<-"),
    ('⟵', "<-"),
    ('↔', "<->"),
    ('⟷', "<->"),
    ('⇒', "==>"),
    ('⟹', "==>"),
    ('⇐', "<=="),
    ('⟸', "<=="),
    ('⇔', "<==>"),
    ('⟺', "<==>"),
    ('≠', "!="),
    ('≤', "<="),
    ('≥', ">="),
    ('≡', "==="),
];

/// A validated scalar-to-text replacement preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreset {
    rules: Vec<TextRule>,
}

impl TextPreset {
    pub fn from_rules(rules: impl IntoIterator<Item = TextRule>) -> Result<Self, TextPresetError> {
        let rules: Vec<_> = rules.into_iter().collect();
        TextEngine::new(rules.iter().cloned()).map_err(TextPresetError::InvalidRules)?;
        if rules.is_empty() {
            return Err(TextPresetError::Empty);
        }
        Ok(Self { rules })
    }

    /// Parses comma-separated `K=V` pairs.
    pub fn parse_inline(input: &str) -> Result<Self, TextPresetError> {
        let pairs = split_unescaped(input, ',', "inline preset")?;
        if pairs.is_empty() || pairs.iter().all(|pair| pair.trim().is_empty()) {
            return Err(TextPresetError::Empty);
        }

        let rules = pairs
            .into_iter()
            .enumerate()
            .map(|(index, pair)| parse_pair(pair, format!("inline pair {}", index + 1)))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_rules(rules)
    }

    /// Parses one `K=V` pair per line. Blank lines and lines beginning with `#`
    /// after optional whitespace are ignored.
    pub fn parse_file(input: &str) -> Result<Self, TextPresetError> {
        let mut rules = Vec::new();
        for (index, raw_line) in input.lines().enumerate() {
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            rules.push(parse_pair(line, format!("line {}", index + 1))?);
        }
        Self::from_rules(rules)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, TextPresetLoadError> {
        let path = path.as_ref();
        let input = fs::read_to_string(path).map_err(|source| TextPresetLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse_file(&input).map_err(|source| TextPresetLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The unchanged AI typography normalization preset from Purra 1.0.
    #[must_use]
    pub fn ai() -> Self {
        let rules = AI_DASHES
            .iter()
            .copied()
            .map(|from| TextRule::new(from, "-"))
            .chain(
                AI_SPACES
                    .iter()
                    .copied()
                    .map(|from| TextRule::new(from, " ")),
            )
            .chain(
                AI_DOUBLE_QUOTES
                    .iter()
                    .copied()
                    .map(|from| TextRule::new(from, "\"")),
            )
            .chain(
                AI_SINGLE_QUOTES
                    .iter()
                    .copied()
                    .map(|from| TextRule::new(from, "'")),
            );

        Self::from_rules(rules).expect("the built-in AI preset must remain valid")
    }

    /// The AI preset plus selected lossless or explicitly chosen ASCII
    /// expansions for typography, arrows, and comparison operators.
    #[must_use]
    pub fn ascii() -> Self {
        let rules = Self::ai().rules.into_iter().chain(
            ASCII_EXPANSIONS
                .iter()
                .map(|&(from, to)| TextRule::new(from, to)),
        );

        Self::from_rules(rules).expect("the built-in ASCII preset must remain valid")
    }

    #[must_use]
    pub fn rules(&self) -> &[TextRule] {
        &self.rules
    }

    #[must_use]
    pub fn engine(&self) -> TextEngine {
        TextEngine::new(self.rules.iter().cloned())
            .expect("a TextPreset always contains valid rules")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TextPresetError {
    #[error("preset must contain at least one rule")]
    Empty,
    #[error("{location}: expected exactly one unescaped '=' separator")]
    InvalidPair { location: String },
    #[error("{location}: dangling escape")]
    DanglingEscape { location: String },
    #[error("{location}: unknown escape \\{escape}")]
    UnknownEscape { location: String, escape: char },
    #[error("{location}: invalid Unicode escape {escape:?}")]
    InvalidUnicodeEscape { location: String, escape: String },
    #[error("{location}: source must decode to exactly one Unicode scalar value")]
    NotOneSourceCharacter { location: String },
    #[error("{location}: replacement must decode to a non-empty string")]
    EmptyReplacement { location: String },
    #[error(transparent)]
    InvalidRules(#[from] TextEngineError),
}

#[derive(Debug, Error)]
pub enum TextPresetLoadError {
    #[error("failed to read preset {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid preset {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: TextPresetError,
    },
}

fn parse_pair(input: &str, location: String) -> Result<TextRule, TextPresetError> {
    let parts = split_unescaped(input, '=', &location)?;
    if parts.len() != 2 {
        return Err(TextPresetError::InvalidPair { location });
    }

    let source_location = format!("{location}, left side");
    let source = decode_text(parts[0].trim(), &source_location)?;
    let mut source_characters = source.chars();
    let Some(from) = source_characters.next() else {
        return Err(TextPresetError::NotOneSourceCharacter {
            location: source_location,
        });
    };
    if source_characters.next().is_some() {
        return Err(TextPresetError::NotOneSourceCharacter {
            location: source_location,
        });
    }

    let replacement_location = format!("{location}, right side");
    let to = decode_text(parts[1].trim(), &replacement_location)?;
    if to.is_empty() {
        return Err(TextPresetError::EmptyReplacement {
            location: replacement_location,
        });
    }
    Ok(TextRule::new(from, to))
}

fn split_unescaped<'a>(
    input: &'a str,
    delimiter: char,
    location: &str,
) -> Result<Vec<&'a str>, TextPresetError> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut escaped = false;

    for (offset, character) in input.char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == delimiter {
            parts.push(&input[start..offset]);
            start = offset + character.len_utf8();
        }
    }

    if escaped {
        return Err(TextPresetError::DanglingEscape {
            location: location.to_owned(),
        });
    }
    parts.push(&input[start..]);
    Ok(parts)
}

fn decode_text(input: &str, location: &str) -> Result<String, TextPresetError> {
    let mut decoded = String::with_capacity(input.len());
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        let Some(escape) = characters.next() else {
            return Err(TextPresetError::DanglingEscape {
                location: location.to_owned(),
            });
        };
        let character = match escape {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            's' => ' ',
            '\\' => '\\',
            '=' => '=',
            ',' => ',',
            '#' => '#',
            'u' => parse_unicode_escape(&mut characters, location)?,
            other => {
                return Err(TextPresetError::UnknownEscape {
                    location: location.to_owned(),
                    escape: other,
                });
            }
        };
        decoded.push(character);
    }

    Ok(decoded)
}

fn parse_unicode_escape(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
    location: &str,
) -> Result<char, TextPresetError> {
    if characters.next() != Some('{') {
        return Err(TextPresetError::InvalidUnicodeEscape {
            location: location.to_owned(),
            escape: "missing opening brace".to_owned(),
        });
    }

    let mut hexadecimal = String::new();
    loop {
        match characters.next() {
            Some('}') if !hexadecimal.is_empty() => break,
            Some(character) if character.is_ascii_hexdigit() && hexadecimal.len() < 6 => {
                hexadecimal.push(character);
            }
            Some(character) => {
                hexadecimal.push(character);
                return Err(TextPresetError::InvalidUnicodeEscape {
                    location: location.to_owned(),
                    escape: hexadecimal,
                });
            }
            None => {
                return Err(TextPresetError::InvalidUnicodeEscape {
                    location: location.to_owned(),
                    escape: hexadecimal,
                });
            }
        }
    }

    let value = u32::from_str_radix(&hexadecimal, 16).ok();
    value
        .and_then(char::from_u32)
        .ok_or(TextPresetError::InvalidUnicodeEscape {
            location: location.to_owned(),
            escape: hexadecimal,
        })
}

#[cfg(test)]
mod tests {
    use super::{ASCII_EXPANSIONS, TextPreset, TextPresetError};

    #[test]
    fn parses_scalar_to_text_rules_and_escapes() {
        let preset = TextPreset::parse_inline(r"…=...,≠=!\=,x=\u{1F1E8}\u{1F1E6}").unwrap();
        let engine = preset.engine();

        assert_eq!(engine.replace("… ≠ x").text, "... != 🇨🇦");
    }

    #[test]
    fn source_remains_exactly_one_scalar_and_replacement_must_not_be_empty() {
        assert!(matches!(
            TextPreset::parse_inline("🇺🇸=flag"),
            Err(TextPresetError::NotOneSourceCharacter { .. })
        ));
        assert!(matches!(
            TextPreset::parse_inline("a="),
            Err(TextPresetError::EmptyReplacement { .. })
        ));
    }

    #[test]
    fn ai_preset_behavior_remains_unchanged() {
        let engine = TextPreset::ai().engine();

        assert_eq!(engine.rule_count(), 53);
        assert_eq!(engine.replace("“—’\u{a0}…⇒").text, "\"-' …⇒");
    }

    #[test]
    fn ascii_preset_contains_ai_rules_and_selected_expansions() {
        let engine = TextPreset::ascii().engine();

        assert_eq!(engine.rule_count(), 53 + ASCII_EXPANSIONS.len());
        assert_eq!(
            engine.replace("“A…B” ﬁ → ⇒ ⇐ ⇔ ≠ ≤ ≥ ≡").text,
            "\"A...B\" fi -> ==> <== <==> != <= >= ==="
        );
        assert_eq!(engine.replace("≈ × ÷ ± ¬ ∧ ∨ Æ Œ ß").count, 0);
    }
}
