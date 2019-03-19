use std::convert::From;
use std::fmt;
use std::fmt::Display;
use std::io;
use std::io::{BufRead, Write};
//use std::iter::FromIterator;

use ansi_term::Colour::RGB;
//use itertools::Itertools;

#[derive(Debug)]
enum LineKind {
    BSD { suffix_start: usize },
    SumPrefixed { prefix_end: usize },
}

#[derive(Debug)]
struct Line {
    contents: String,
    kind: Option<LineKind>,
}

#[derive(Debug)]
struct Colour {
    red: u8,
    green: u8,
    blue: u8,
}

impl From<String> for Line {
    fn from(contents: String) -> Self {
        let mut kind: Option<LineKind> = None;

        if let Some(suffix_start) = find_bsd_tag_line(&contents) {
            kind = Some(LineKind::BSD { suffix_start });
        } else if let Some(prefix_end) = find_sum_prefixed_line(&contents) {
            kind = Some(LineKind::SumPrefixed { prefix_end });
        }

        Self { contents, kind }
    }
}

impl Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        // Fall back to writing with no extra formatting
        // if we didn't detect any particular kind of input
        let line_kind = match &self.kind {
            None => return write!(formatter, "{}", self.contents),
            Some(kind) => kind,
        };

        match line_kind {
            LineKind::BSD { suffix_start } => {
                let (prefix, suffix) = self.contents.split_at(*suffix_start);
                write!(formatter, "{}{}", prefix, format_hash(suffix.to_string()))
            }
            LineKind::SumPrefixed { prefix_end } => {
                let (prefix, suffix) = self.contents.split_at(*prefix_end);
                write!(formatter, "{}{}", format_hash(prefix.to_string()), suffix)
            }
        }
    }
}

/// Formats a base16-format hash or digest.
///
/// Each 8-bit hexadecimal digit will be coloured
/// with the corresponding xterm colour.
fn format_hash(hash: String) -> String {
    use std::num::ParseIntError;

    // map over every two characters
    let result: Result<String, ParseIntError> = hash
        .chars()
        .into_iter()
        .map(|c| {

            let mut colour: Colour = Colour {
                red: 255,
                green: 156,
                blue: 73
            };

            if c.is_ascii_digit() {
                colour = Colour {
                    red: 121,
                    green: 176,
                    blue: 215
                };
            }

            let output = c.to_string();

            // TODO: I need to rewrite this flow
            u8::from_str_radix(&output, 16)
                .map(|_ordinal|  RGB(colour.red, colour.green, colour.blue).paint(&output).to_string())
        })
        .collect();


    // if there was an error at any point, return the original value
    result.unwrap_or(hash)
}

/// Detects the *starting* offset of the
/// hash in a BSD `md5(1)` style line
fn find_bsd_tag_line(line: &str) -> Option<usize> {
    let needle = " = ";
    line.rfind(needle).map(|offset| offset + needle.len())
}

/// Detects the *ending* offset of the hash in a
/// GNU `md5sum(1)` / perl `shasum(1)` style line
fn find_sum_prefixed_line(line: &str) -> Option<usize> {
    line.find("  ")
}

/// Takes each line in `from`, and writes it to `to`.
///
/// If a given line is recognisable as the output of a
/// hashing utility, its hash value will be coloured.
fn coloursum<F: BufRead, T: Write>(from: F, mut to: T) -> io::Result<()> {
    for wrapped_line in from.lines() {
        writeln!(to, "{}", Line::from(wrapped_line?))?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let locked_stdin = stdin.lock();

    let stdout = io::stdout();
    let locked_stdout = stdout.lock();

    coloursum(locked_stdin, locked_stdout)
}

#[cfg(test)]
mod tests {
    #[test]
    fn format_hash_works() {
        use super::format_hash;

        assert_eq!(
            format_hash(
                "b7527e0e28c09f6f62dd2d4197d5d225".to_string()
            ),
            "\u{1b}[38;5;183mb7\u{1b}[0m\u{1b}[38;5;82m52\u{1b}[0m\u{1b}[38;5;126m7e\u{1b}[0m\u{1b}[38;5;14m0e\u{1b}[0m\u{1b}[38;5;40m28\u{1b}[0m\u{1b}[38;5;192mc0\u{1b}[0m\u{1b}[38;5;159m9f\u{1b}[0m\u{1b}[38;5;111m6f\u{1b}[0m\u{1b}[38;5;98m62\u{1b}[0m\u{1b}[38;5;221mdd\u{1b}[0m\u{1b}[38;5;45m2d\u{1b}[0m\u{1b}[38;5;65m41\u{1b}[0m\u{1b}[38;5;151m97\u{1b}[0m\u{1b}[38;5;213md5\u{1b}[0m\u{1b}[38;5;210md2\u{1b}[0m\u{1b}[38;5;37m25\u{1b}[0m"
        );
    }

    #[test]
    fn format_hash_doesnt_crash_on_non_base16_characters() {
        use super::format_hash;

        format_hash("ASDF".to_string());
        format_hash("😄".to_string());
    }

    #[test]
    fn find_bsd_tag_line_works() {
        use super::find_bsd_tag_line;

        assert_eq!(
            find_bsd_tag_line(
                &"MD5 (./src/main.rs) = b7527e0e28c09f6f62dd2d4197d5d225".to_string()
            ),
            Some(22)
        );
        assert_eq!(
            find_bsd_tag_line(&"b7527e0e28c09f6f62dd2d4197d5d225  ./src/main.rs".to_string()),
            None
        );
        assert_eq!(
            find_bsd_tag_line(
                &"3e08ba70bfc57da75612af458c7ea94108f9a9ddf9d1bfd96de9c0e34e684bda  ./src/main.rs"
                    .to_string()
            ),
            None
        );
    }

    #[test]
    fn find_sum_prefixed_line_works() {
        use super::find_sum_prefixed_line;

        assert_eq!(
            find_sum_prefixed_line(&"b7527e0e28c09f6f62dd2d4197d5d225  ./src/main.rs".to_string()),
            Some(32)
        );
        assert_eq!(
            find_sum_prefixed_line(
                &"3e08ba70bfc57da75612af458c7ea94108f9a9ddf9d1bfd96de9c0e34e684bda  ./src/main.rs"
                    .to_string()
            ),
            Some(64)
        );
        assert_eq!(
            find_sum_prefixed_line(
                &"MD5 (./src/main.rs) = b7527e0e28c09f6f62dd2d4197d5d225".to_string()
            ),
            None
        );
    }
}
