//! Various utility functions.

use colored::{ColoredString, Colorize};
use rustyline::Editor;
use std::io::Write;
use std::str::FromStr;
use unic_langid::LanguageIdentifier;

/// Injection of the `void` method into [`Result`], which is a common shorthand
/// for "forgetting" the internal return value of a `Result`. Note that this
/// also automatically lifts the Error type via [`From`], as it is intended as
/// the final line of a function where `?` doesn't work.
///
/// We assume that only `Result` is ever going to implement this trait. If Rust
/// had Higher-kinded Types, this would be much simpler and could be applied to
/// more types.
pub(crate) trait ResultVoid<E, R> {
    fn void(self) -> Result<(), R>
    where
        R: From<E>;
}

impl<T, E, R> ResultVoid<E, R> for Result<T, E> {
    fn void(self) -> Result<(), R>
    where
        R: From<E>,
    {
        match self {
            Ok(_) => Ok(()),
            Err(e) => Err(From::from(e)),
        }
    }
}

/// A helper for commands like `-Ai`, `-Ci`, etc.
pub(crate) fn info<W>(
    w: &mut W,
    lang: LanguageIdentifier,
    pairs: &[(&str, ColoredString)],
) -> Result<(), std::io::Error>
where
    W: Write,
{
    // Different languages consume varying char widths in the terminal.
    //
    // TODO Account for other languages (Chinese, and what else?)
    let m = if lang.language == "ja" { 2 } else { 1 };

    // The longest field.
    let l = pairs
        .iter()
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(0);

    for (lbl, value) in pairs {
        writeln!(w, "{}{:w$} : {}", lbl.bold(), "", value, w = pad(m, l, lbl))?;
    }

    Ok(())
}

fn pad(mult: usize, longest: usize, s: &str) -> usize {
    mult * (longest - s.chars().count())
}

// TODO Localize the acceptance chars.
/// Prompt the user for confirmation.
pub(crate) fn prompt(msg: &str) -> Option<()> {
    let mut rl = Editor::<()>::new();
    let line = rl.readline(msg).ok()?;

    (line.is_empty() || line == "y" || line == "Y").then(|| ())
}

/// Prompt the user for a numerical selection.
pub(crate) fn select(msg: &str, max: usize) -> Result<usize, rustyline::error::ReadlineError> {
    let mut rl = Editor::<()>::new();

    loop {
        let raw = rl.readline(msg)?;

        if let Ok(num) = usize::from_str(&raw) {
            if max >= num {
                return Ok(num);
            }
        }
    }
}

pub struct SudoError;

impl std::fmt::Display for SudoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to raise privileges.")
    }
}

/// Escalate the privileges of the Aura process, if necessary.
pub(crate) fn sudo() -> Result<(), SudoError> {
    sudo::escalate_if_needed().map_err(|_| SudoError).void()
}
