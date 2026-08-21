use std::borrow::Cow;
use std::collections::HashMap;

use thiserror::Error;

/// A single non-transitive Unicode scalar-value replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rule {
    pub from: char,
    pub to: char,
}

impl Rule {
    #[must_use]
    pub const fn new(from: char, to: char) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("duplicate source character {character:?} in rules {first} and {second}")]
    DuplicateSource {
        character: char,
        first: usize,
        second: usize,
    },
    #[error("duplicate destination character {character:?} in rules {first} and {second}")]
    DuplicateDestination {
        character: char,
        first: usize,
        second: usize,
    },
    #[error("rule {position} maps {character:?} to itself")]
    NoOp { character: char, position: usize },
}

/// The result of a replacement operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement<'a> {
    pub text: Cow<'a, str>,
    pub count: usize,
}

impl Replacement<'_> {
    #[must_use]
    pub const fn changed(&self) -> bool {
        self.count != 0
    }
}

/// A replacement found in the original input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Finding {
    pub original: char,
    pub replacement: char,
    /// Zero-based byte offset in the original UTF-8 input.
    pub byte_offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column counted in Unicode scalar values, not grapheme clusters.
    pub column: usize,
}

/// A compiled, reusable character replacement engine.
///
/// ASCII rules use a direct lookup table. Non-ASCII rules use a sorted slice
/// with binary search, keeping lookup deterministic without allocating per input.
#[derive(Debug, Clone)]
pub struct Engine {
    ascii: [Option<char>; 128],
    unicode: Box<[(char, char)]>,
    rule_count: usize,
}

impl Engine {
    pub fn new(rules: impl IntoIterator<Item = Rule>) -> Result<Self, EngineError> {
        let rules: Vec<_> = rules.into_iter().collect();
        validate_rules(&rules)?;

        let mut ascii = [None; 128];
        let mut unicode = Vec::with_capacity(rules.len());
        for rule in &rules {
            if rule.from.is_ascii() {
                ascii[rule.from as usize] = Some(rule.to);
            } else {
                unicode.push((rule.from, rule.to));
            }
        }
        unicode.sort_unstable_by_key(|(from, _)| *from);

        Ok(Self {
            ascii,
            unicode: unicode.into_boxed_slice(),
            rule_count: rules.len(),
        })
    }

    #[must_use]
    pub const fn rule_count(&self) -> usize {
        self.rule_count
    }

    #[must_use]
    pub fn replacement_for(&self, character: char) -> Option<char> {
        if character.is_ascii() {
            self.ascii[character as usize]
        } else {
            self.unicode
                .binary_search_by_key(&character, |(from, _)| *from)
                .ok()
                .map(|index| self.unicode[index].1)
        }
    }

    /// Replaces all configured characters in one pass.
    ///
    /// The returned text borrows `input` when no replacements are needed.
    #[must_use]
    pub fn replace<'a>(&self, input: &'a str) -> Replacement<'a> {
        let first = input
            .char_indices()
            .find_map(|(offset, character)| self.replacement_for(character).map(|to| (offset, to)));

        let Some((first_offset, first_replacement)) = first else {
            return Replacement {
                text: Cow::Borrowed(input),
                count: 0,
            };
        };

        let mut output = String::with_capacity(input.len());
        output.push_str(&input[..first_offset]);
        output.push(first_replacement);

        let first_len = input[first_offset..]
            .chars()
            .next()
            .expect("first replacement points at a character")
            .len_utf8();
        let remainder_offset = first_offset + first_len;
        let mut count = 1;

        for character in input[remainder_offset..].chars() {
            if let Some(replacement) = self.replacement_for(character) {
                output.push(replacement);
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
    pub fn find(&self, input: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut line = 1;
        let mut column = 1;

        for (byte_offset, original) in input.char_indices() {
            if let Some(replacement) = self.replacement_for(original) {
                findings.push(Finding {
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

fn validate_rules(rules: &[Rule]) -> Result<(), EngineError> {
    let mut sources = HashMap::with_capacity(rules.len());
    let mut destinations = HashMap::with_capacity(rules.len());

    for (index, rule) in rules.iter().enumerate() {
        let position = index + 1;
        if rule.from == rule.to {
            return Err(EngineError::NoOp {
                character: rule.from,
                position,
            });
        }
        if let Some(first) = sources.insert(rule.from, position) {
            return Err(EngineError::DuplicateSource {
                character: rule.from,
                first,
                second: position,
            });
        }
        if let Some(first) = destinations.insert(rule.to, position) {
            return Err(EngineError::DuplicateDestination {
                character: rule.to,
                first,
                second: position,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{Engine, EngineError, Rule};

    #[test]
    fn unchanged_text_stays_borrowed() {
        let engine = Engine::new([Rule::new('—', '-')]).unwrap();
        let result = engine.replace("plain text");

        assert_eq!(result.text, "plain text");
        assert_eq!(result.count, 0);
        assert!(matches!(result.text, Cow::Borrowed(_)));
    }

    #[test]
    fn replacement_is_non_transitive() {
        let engine = Engine::new([Rule::new('a', 'b'), Rule::new('b', 'c')]).unwrap();

        assert_eq!(engine.replace("ab").text, "bc");
    }

    #[test]
    fn supports_different_utf8_widths() {
        let engine = Engine::new([
            Rule::new('—', '-'),
            Rule::new('a', '界'),
            Rule::new('\u{301}', '^'),
        ])
        .unwrap();

        assert_eq!(engine.replace("a—e\u{301}").text, "界-e^");
    }

    #[test]
    fn treats_a_flag_as_two_unicode_scalars() {
        let engine = Engine::new([Rule::new('🇺', '🇨'), Rule::new('🇸', '🇦')]).unwrap();
        let result = engine.replace("flag: 🇺🇸");

        assert_eq!(result.text, "flag: 🇨🇦");
        assert_eq!(result.count, 2);
    }

    #[test]
    fn reports_scalar_columns_and_byte_offsets() {
        let engine = Engine::new([Rule::new('🇺', '🇨'), Rule::new('🇸', '🇦')]).unwrap();
        let findings = engine.find("é🇺🇸\n🇺");

        assert_eq!(findings[0].byte_offset, 2);
        assert_eq!((findings[0].line, findings[0].column), (1, 2));
        assert_eq!(findings[1].byte_offset, 6);
        assert_eq!((findings[1].line, findings[1].column), (1, 3));
        assert_eq!((findings[2].line, findings[2].column), (2, 1));
    }

    #[test]
    fn rejects_duplicate_sources_and_destinations() {
        assert!(matches!(
            Engine::new([Rule::new('a', 'b'), Rule::new('a', 'c')]),
            Err(EngineError::DuplicateSource { .. })
        ));
        assert!(matches!(
            Engine::new([Rule::new('a', 'c'), Rule::new('b', 'c')]),
            Err(EngineError::DuplicateDestination { .. })
        ));
        assert!(matches!(
            Engine::new([Rule::new('a', 'a')]),
            Err(EngineError::NoOp { .. })
        ));
    }
}
