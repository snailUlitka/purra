use std::borrow::Cow;
use std::collections::HashMap;

use thiserror::Error;

use crate::Replacement;

/// A non-transitive replacement from one Unicode scalar value to non-empty text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextRule {
    pub from: char,
    pub to: Box<str>,
}

impl TextRule {
    #[must_use]
    pub fn new(from: char, to: impl Into<Box<str>>) -> Self {
        Self {
            from,
            to: to.into(),
        }
    }
}

#[allow(deprecated)]
impl From<crate::Rule> for TextRule {
    fn from(rule: crate::Rule) -> Self {
        Self::new(rule.from, rule.to.to_string())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TextEngineError {
    #[error("duplicate source character {character:?} in rules {first} and {second}")]
    DuplicateSource {
        character: char,
        first: usize,
        second: usize,
    },
    #[error("rule {position} maps {character:?} to itself")]
    NoOp { character: char, position: usize },
    #[error("rule {position} has an empty replacement for {character:?}")]
    EmptyReplacement { character: char, position: usize },
}

/// A text replacement found in the original input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFinding<'a> {
    pub original: char,
    pub replacement: &'a str,
    /// Zero-based byte offset in the original UTF-8 input.
    pub byte_offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column counted in Unicode scalar values, not grapheme clusters.
    pub column: usize,
}

/// A compiled, reusable scalar-to-text replacement engine.
///
/// ASCII sources use a direct lookup table. Non-ASCII sources use a sorted
/// slice with binary search. Replacement text is never processed again during
/// the same pass.
#[derive(Debug, Clone)]
pub struct TextEngine {
    ascii: [Option<Box<str>>; 128],
    unicode: Box<[(char, Box<str>)]>,
    rule_count: usize,
}

impl TextEngine {
    pub fn new(rules: impl IntoIterator<Item = TextRule>) -> Result<Self, TextEngineError> {
        let rules: Vec<_> = rules.into_iter().collect();
        validate_rules(&rules)?;

        let mut ascii = std::array::from_fn(|_| None);
        let mut unicode = Vec::with_capacity(rules.len());
        for rule in rules {
            if rule.from.is_ascii() {
                ascii[rule.from as usize] = Some(rule.to);
            } else {
                unicode.push((rule.from, rule.to));
            }
        }
        unicode.sort_unstable_by_key(|(from, _)| *from);
        let rule_count = ascii_rule_count(&ascii) + unicode.len();

        Ok(Self {
            ascii,
            unicode: unicode.into_boxed_slice(),
            rule_count,
        })
    }

    #[must_use]
    pub const fn rule_count(&self) -> usize {
        self.rule_count
    }

    #[must_use]
    pub fn replacement_for(&self, character: char) -> Option<&str> {
        if character.is_ascii() {
            self.ascii[character as usize].as_deref()
        } else {
            self.unicode
                .binary_search_by_key(&character, |(from, _)| *from)
                .ok()
                .map(|index| self.unicode[index].1.as_ref())
        }
    }

    /// Replaces all configured source characters in one pass.
    ///
    /// The returned text borrows `input` when no replacements are needed.
    #[must_use]
    pub fn replace<'a>(&self, input: &'a str) -> Replacement<'a> {
        let first = input.char_indices().find_map(|(offset, character)| {
            self.replacement_for(character)
                .map(|replacement| (offset, character.len_utf8(), replacement))
        });

        let Some((first_offset, first_len, first_replacement)) = first else {
            return Replacement {
                text: Cow::Borrowed(input),
                count: 0,
            };
        };

        let mut output = String::with_capacity(input.len());
        output.push_str(&input[..first_offset]);
        output.push_str(first_replacement);

        let remainder_offset = first_offset + first_len;
        let mut count = 1;
        for character in input[remainder_offset..].chars() {
            if let Some(replacement) = self.replacement_for(character) {
                output.push_str(replacement);
                count += 1;
            } else {
                output.push(character);
            }
        }

        Replacement {
            text: Cow::Owned(output),
            count,
        }
    }

    /// Returns findings in source order without producing replacement text.
    #[must_use]
    pub fn find<'a>(&'a self, input: &str) -> Vec<TextFinding<'a>> {
        let mut findings = Vec::new();
        let mut line = 1;
        let mut column = 1;

        for (byte_offset, original) in input.char_indices() {
            if let Some(replacement) = self.replacement_for(original) {
                findings.push(TextFinding {
                    original,
                    replacement,
                    byte_offset,
                    line,
                    column,
                });
            }

            if original == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        findings
    }
}

fn ascii_rule_count(ascii: &[Option<Box<str>>; 128]) -> usize {
    ascii
        .iter()
        .filter(|replacement| replacement.is_some())
        .count()
}

fn validate_rules(rules: &[TextRule]) -> Result<(), TextEngineError> {
    let mut sources = HashMap::with_capacity(rules.len());

    for (index, rule) in rules.iter().enumerate() {
        let position = index + 1;
        if rule.to.is_empty() {
            return Err(TextEngineError::EmptyReplacement {
                character: rule.from,
                position,
            });
        }
        if replacement_is_same_scalar(rule.from, &rule.to) {
            return Err(TextEngineError::NoOp {
                character: rule.from,
                position,
            });
        }
        if let Some(first) = sources.insert(rule.from, position) {
            return Err(TextEngineError::DuplicateSource {
                character: rule.from,
                first,
                second: position,
            });
        }
    }
    Ok(())
}

fn replacement_is_same_scalar(source: char, replacement: &str) -> bool {
    let mut characters = replacement.chars();
    characters.next() == Some(source) && characters.next().is_none()
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{TextEngine, TextEngineError, TextRule};

    #[test]
    fn unchanged_text_stays_borrowed() {
        let engine = TextEngine::new([TextRule::new('…', "...")]).unwrap();
        let result = engine.replace("plain text");

        assert_eq!(result.text, "plain text");
        assert_eq!(result.count, 0);
        assert!(matches!(result.text, Cow::Borrowed(_)));
    }

    #[test]
    fn supports_expanding_replacements_without_transitivity() {
        let engine = TextEngine::new([
            TextRule::new('a', "bc"),
            TextRule::new('b', "d"),
            TextRule::new('…', "..."),
        ])
        .unwrap();

        let result = engine.replace("ab…");
        assert_eq!(result.text, "bcd...");
        assert_eq!(result.count, 3);
    }

    #[test]
    fn reports_scalar_columns_and_text_replacements() {
        let engine = TextEngine::new([TextRule::new('…', "...")]).unwrap();
        let findings = engine.find("é…\n…");

        assert_eq!(findings[0].byte_offset, 2);
        assert_eq!((findings[0].line, findings[0].column), (1, 2));
        assert_eq!(findings[0].replacement, "...");
        assert_eq!((findings[1].line, findings[1].column), (2, 1));
    }

    #[test]
    fn rejects_empty_no_op_and_duplicate_rules() {
        assert!(matches!(
            TextEngine::new([TextRule::new('a', "")]),
            Err(TextEngineError::EmptyReplacement { .. })
        ));
        assert!(matches!(
            TextEngine::new([TextRule::new('a', "a")]),
            Err(TextEngineError::NoOp { .. })
        ));
        assert!(matches!(
            TextEngine::new([TextRule::new('a', "b"), TextRule::new('a', "bc")]),
            Err(TextEngineError::DuplicateSource { .. })
        ));
    }
}
