use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::engine::{Engine, EngineError, Rule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    rules: Vec<Rule>,
}

impl Preset {
    pub fn from_rules(rules: impl IntoIterator<Item = Rule>) -> Result<Self, PresetError> {
        let rules: Vec<_> = rules.into_iter().collect();
        Engine::new(rules.iter().copied()).map_err(PresetError::InvalidRules)?;
        if rules.is_empty() {
            return Err(PresetError::Empty);
        }
        Ok(Self { rules })
    }

    /// Parses comma-separated `K=V` pairs.
    pub fn parse_inline(input: &str) -> Result<Self, PresetError> {
        let pairs = split_unescaped(input, ',', "inline preset")?;
        if pairs.is_empty() || pairs.iter().all(|pair| pair.trim().is_empty()) {
            return Err(PresetError::Empty);
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
    pub fn parse_file(input: &str) -> Result<Self, PresetError> {
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

    pub fn load(path: impl AsRef<Path>) -> Result<Self, PresetLoadError> {
        let path = path.as_ref();
        let input = fs::read_to_string(path).map_err(|source| PresetLoadError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse_file(&input).map_err(|source| PresetLoadError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    /// The conservative built-in AI typography preset shipped with Purra.
    #[must_use]
    pub fn ai() -> Self {
        Self::from_rules([
            Rule::new('—', '-'),
            Rule::new('\u{a0}', ' '),
            Rule::new('“', '"'),
            Rule::new('’', '\''),
        ])
        .expect("the built-in AI preset must remain valid")
    }

    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    #[must_use]
    pub fn engine(&self) -> Engine {
        Engine::new(self.rules.iter().copied()).expect("a Preset always contains valid rules")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PresetError {
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
    #[error("{location}: each side must decode to exactly one Unicode scalar value")]
    NotOneCharacter { location: String },
    #[error(transparent)]
    InvalidRules(#[from] EngineError),
}

#[derive(Debug, Error)]
pub enum PresetLoadError {
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
        source: PresetError,
    },
}

fn parse_pair(input: &str, location: String) -> Result<Rule, PresetError> {
    let parts = split_unescaped(input, '=', &location)?;
    if parts.len() != 2 {
        return Err(PresetError::InvalidPair { location });
    }

    let from = decode_character(parts[0].trim(), format!("{location}, left side"))?;
    let to = decode_character(parts[1].trim(), format!("{location}, right side"))?;
    Ok(Rule::new(from, to))
}

fn split_unescaped<'a>(
    input: &'a str,
    delimiter: char,
    location: &str,
) -> Result<Vec<&'a str>, PresetError> {
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
        return Err(PresetError::DanglingEscape {
            location: location.to_owned(),
        });
    }
    parts.push(&input[start..]);
    Ok(parts)
}

fn decode_character(input: &str, location: String) -> Result<char, PresetError> {
    let mut decoded = Vec::with_capacity(2);
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        let Some(escape) = characters.next() else {
            return Err(PresetError::DanglingEscape { location });
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
            'u' => parse_unicode_escape(&mut characters, &location)?,
            other => {
                return Err(PresetError::UnknownEscape {
                    location,
                    escape: other,
                });
            }
        };
        decoded.push(character);
    }

    if decoded.len() != 1 {
        return Err(PresetError::NotOneCharacter { location });
    }
    Ok(decoded[0])
}

fn parse_unicode_escape(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
    location: &str,
) -> Result<char, PresetError> {
    if characters.next() != Some('{') {
        return Err(PresetError::InvalidUnicodeEscape {
            location: location.to_owned(),
            escape: "expected \\u{HEX}".to_owned(),
        });
    }

    let mut hexadecimal = String::new();
    loop {
        match characters.next() {
            Some('}') => break,
            Some(character) if character.is_ascii_hexdigit() && hexadecimal.len() < 6 => {
                hexadecimal.push(character);
            }
            Some(character) => {
                hexadecimal.push(character);
                return Err(PresetError::InvalidUnicodeEscape {
                    location: location.to_owned(),
                    escape: hexadecimal,
                });
            }
            None => {
                return Err(PresetError::InvalidUnicodeEscape {
                    location: location.to_owned(),
                    escape: hexadecimal,
                });
            }
        }
    }

    let value = u32::from_str_radix(&hexadecimal, 16).ok();
    value
        .and_then(char::from_u32)
        .ok_or(PresetError::InvalidUnicodeEscape {
            location: location.to_owned(),
            escape: hexadecimal,
        })
}

#[cfg(test)]
mod tests {
    use super::{Preset, PresetError};

    #[test]
    fn parses_inline_literals_and_escapes() {
        let preset = Preset::parse_inline(r"a=b,\,=\=,\s=_,$=\u{20AC}").unwrap();
        let engine = preset.engine();

        assert_eq!(engine.replace("a, $ ").text, "b=_€_");
    }

    #[test]
    fn parses_file_comments_blank_lines_and_crlf() {
        let preset = Preset::parse_file("# typography\r\n\r\n—=-\r\n\\s=_\r\n").unwrap();

        assert_eq!(preset.engine().replace("a — b").text, "a_-_b");
    }

    #[test]
    fn rejects_a_whole_flag_because_it_has_two_scalars() {
        let error = Preset::parse_inline("🇺🇸=🇨🇦").unwrap_err();

        assert!(matches!(error, PresetError::NotOneCharacter { .. }));
    }

    #[test]
    fn accepts_each_regional_indicator_as_a_character() {
        let preset = Preset::parse_inline("🇺=🇨,🇸=🇦").unwrap();

        assert_eq!(preset.engine().replace("🇺🇸").text, "🇨🇦");
    }

    #[test]
    fn rejects_duplicate_keys_values_and_bad_syntax() {
        assert!(Preset::parse_inline("a=b,a=c").is_err());
        assert!(Preset::parse_inline("a=c,b=c").is_err());
        assert!(Preset::parse_inline("ab=c").is_err());
        assert!(Preset::parse_inline("a=b=c").is_err());
        assert!(Preset::parse_inline("").is_err());
        assert!(Preset::parse_inline(r"a=\q").is_err());
        assert!(Preset::parse_inline(r"a=\u{D800}").is_err());
    }

    #[test]
    fn built_in_preset_has_expected_typography() {
        let engine = Preset::ai().engine();

        assert_eq!(engine.replace("“—’\u{a0}").text, "\"-' ");
    }
}
