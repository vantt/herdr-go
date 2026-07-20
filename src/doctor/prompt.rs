//! TTY detection and interactive prompt primitives for `doctor`.
//!
//! Per D15, prompts are written to stderr — never stdout — so a piped stdout
//! still carries only the report. Nothing here is wired into a check yet; that
//! is Slice 2's job.

use std::io::{self, BufRead, IsTerminal, Write};

/// True only when both stdin and stderr are terminals (D15). A non-interactive
/// pipe on either stream means doctor must not prompt.
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Ask a yes/no question with a default answer used when the user enters nothing.
pub fn confirm(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: &str,
    default: bool,
) -> io::Result<bool> {
    let hint = if default { "Y/n" } else { "y/N" };
    loop {
        write!(writer, "{prompt} [{hint}] ")?;
        writer.flush()?;
        let line = read_line(reader)?;
        let line = line.trim();
        if line.is_empty() {
            return Ok(default);
        }
        match line.to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => writeln!(writer, "please answer y or n")?,
        }
    }
}

/// Ask for a line of text, returning `default` when the input is empty.
pub fn prompt_line(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: &str,
    default: Option<&str>,
) -> io::Result<String> {
    match default {
        Some(d) => write!(writer, "{prompt} [{d}] ")?,
        None => write!(writer, "{prompt} ")?,
    }
    writer.flush()?;
    let line = read_line(reader)?;
    let line = line.trim();
    if line.is_empty() {
        Ok(default.unwrap_or("").to_string())
    } else {
        Ok(line.to_string())
    }
}

/// Ask the user to type an exact phrase to confirm a sensitive action (e.g.
/// widening `allowed_roots`). Returns true only on an exact match.
pub fn typed_confirm(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: &str,
    expected: &str,
) -> io::Result<bool> {
    write!(writer, "{prompt} (type \"{expected}\" to confirm) ")?;
    writer.flush()?;
    let line = read_line(reader)?;
    Ok(line.trim() == expected)
}

/// Present a numbered list of options and return the chosen index (0-based),
/// defaulting to `default` when the input is empty.
pub fn choose(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    prompt: &str,
    options: &[&str],
    default: usize,
) -> io::Result<usize> {
    writeln!(writer, "{prompt}")?;
    for (i, opt) in options.iter().enumerate() {
        writeln!(writer, "  {}) {}", i + 1, opt)?;
    }
    loop {
        write!(
            writer,
            "choose [1-{}, default {}] ",
            options.len(),
            default + 1
        )?;
        writer.flush()?;
        let line = read_line(reader)?;
        let line = line.trim();
        if line.is_empty() {
            return Ok(default);
        }
        match line.parse::<usize>() {
            Ok(n) if n >= 1 && n <= options.len() => return Ok(n - 1),
            _ => writeln!(writer, "enter a number between 1 and {}", options.len())?,
        }
    }
}

fn read_line(reader: &mut impl BufRead) -> io::Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line)
}

/// The result of a masked secret prompt. Empty input is an explicit skip
/// (D13) rather than an empty secret, so it never blocks progress.
pub enum SecretEntry {
    Value(String),
    Skipped,
}

impl SecretEntry {
    /// A display form that never reveals the full secret (D13): length, plus
    /// the last 3 characters only when the value is at least 12 characters
    /// long. Leading characters are never shown.
    pub fn display(&self) -> String {
        match self {
            SecretEntry::Skipped => "skipped".to_string(),
            SecretEntry::Value(v) => {
                let len = v.chars().count();
                if len >= 12 {
                    let tail: String = v.chars().skip(len - 3).collect();
                    format!("{len} chars, ends \"{tail}\"")
                } else {
                    format!("{len} chars")
                }
            }
        }
    }
}

/// Prompt for a secret with terminal echo suppressed, using `rpassword` (D13,
/// the one new dependency this feature adds). The prompt label is written to
/// `writer` (stderr in production); the value itself is read via `config`.
pub fn prompt_secret_with_config(
    writer: &mut impl Write,
    prompt: &str,
    config: rpassword::Config,
) -> io::Result<SecretEntry> {
    write!(writer, "{prompt} (input hidden, empty to skip) ")?;
    writer.flush()?;
    let raw = rpassword::read_password_with_config(config)?;
    Ok(if raw.is_empty() {
        SecretEntry::Skipped
    } else {
        SecretEntry::Value(raw)
    })
}

/// Production entry point for [`prompt_secret_with_config`]: reads from the
/// real TTY with echo suppressed.
pub fn prompt_secret(writer: &mut impl Write, prompt: &str) -> io::Result<SecretEntry> {
    prompt_secret_with_config(writer, prompt, rpassword::ConfigBuilder::default().build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn reader(input: &str) -> Cursor<Vec<u8>> {
        Cursor::new(input.as_bytes().to_vec())
    }

    #[test]
    fn confirm_empty_input_takes_default() {
        let mut r = reader("\n");
        let mut w = Vec::new();
        assert!(confirm(&mut r, &mut w, "proceed?", true).unwrap());

        let mut r = reader("\n");
        let mut w = Vec::new();
        assert!(!confirm(&mut r, &mut w, "proceed?", false).unwrap());
    }

    #[test]
    fn confirm_accepts_y_and_n() {
        let mut r = reader("yes\n");
        let mut w = Vec::new();
        assert!(confirm(&mut r, &mut w, "proceed?", false).unwrap());

        let mut r = reader("n\n");
        let mut w = Vec::new();
        assert!(!confirm(&mut r, &mut w, "proceed?", true).unwrap());
    }

    #[test]
    fn confirm_reprompts_on_invalid_input() {
        let mut r = reader("maybe\ny\n");
        let mut w = Vec::new();
        assert!(confirm(&mut r, &mut w, "proceed?", false).unwrap());
        let out = String::from_utf8(w).unwrap();
        assert!(out.contains("please answer y or n"));
    }

    #[test]
    fn prompt_line_empty_takes_default() {
        let mut r = reader("\n");
        let mut w = Vec::new();
        assert_eq!(
            prompt_line(&mut r, &mut w, "root:", Some("/data")).unwrap(),
            "/data"
        );
    }

    #[test]
    fn prompt_line_uses_typed_value() {
        let mut r = reader("/srv/media\n");
        let mut w = Vec::new();
        assert_eq!(
            prompt_line(&mut r, &mut w, "root:", Some("/data")).unwrap(),
            "/srv/media"
        );
    }

    #[test]
    fn typed_confirm_requires_exact_match() {
        let mut r = reader("widen\n");
        let mut w = Vec::new();
        assert!(typed_confirm(&mut r, &mut w, "allow /?", "widen").unwrap());

        let mut r = reader("no\n");
        let mut w = Vec::new();
        assert!(!typed_confirm(&mut r, &mut w, "allow /?", "widen").unwrap());
    }

    #[test]
    fn choose_empty_takes_default() {
        let mut r = reader("\n");
        let mut w = Vec::new();
        assert_eq!(
            choose(&mut r, &mut w, "pick one", &["a", "b", "c"], 1).unwrap(),
            1
        );
    }

    #[test]
    fn choose_reprompts_on_out_of_range() {
        let mut r = reader("9\n2\n");
        let mut w = Vec::new();
        assert_eq!(
            choose(&mut r, &mut w, "pick one", &["a", "b", "c"], 0).unwrap(),
            1
        );
    }

    #[test]
    fn secret_entry_display_never_reveals_short_value() {
        let entry = SecretEntry::Value("tok-1234".to_string());
        let shown = entry.display();
        assert_eq!(shown, "8 chars");
        assert!(!shown.contains("tok-1234"));
    }

    #[test]
    fn secret_entry_display_shows_tail_only_when_long_enough() {
        let entry = SecretEntry::Value("abcdefghijklmnop".to_string());
        let shown = entry.display();
        assert_eq!(shown, "16 chars, ends \"nop\"");
        assert!(!shown.contains("abcdefghijklm"));
    }

    #[test]
    fn secret_entry_display_skipped() {
        assert_eq!(SecretEntry::Skipped.display(), "skipped");
    }

    #[test]
    fn prompt_secret_empty_input_is_skipped() {
        let mut w = Vec::new();
        let config = rpassword::ConfigBuilder::new()
            .input_data("\n")
            .output_discard()
            .build();
        let entry = prompt_secret_with_config(&mut w, "token:", config).unwrap();
        assert!(matches!(entry, SecretEntry::Skipped));
    }

    #[test]
    fn prompt_secret_reads_value_without_echo() {
        let mut w = Vec::new();
        let config = rpassword::ConfigBuilder::new()
            .input_data("super-secret-token\n")
            .output_discard()
            .build();
        let entry = prompt_secret_with_config(&mut w, "token:", config).unwrap();
        match entry {
            SecretEntry::Value(v) => assert_eq!(v, "super-secret-token"),
            SecretEntry::Skipped => panic!("expected a value"),
        }
    }
}
