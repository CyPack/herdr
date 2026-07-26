//! `herdr view <PATH> [--page N]` — show one file in this pane.
//!
//! Argument parsing only; the viewer itself lives in [`crate::viewer`]. Kept
//! separate so the parse rules are testable without a terminal.

use std::path::PathBuf;

/// What the command line asked for, once it has been understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ViewArgs {
    pub(super) path: PathBuf,
    /// Zero-based, as every page index in herdr is. `--page` is one-based
    /// because that is what the viewer prints, and asking a reader to type a
    /// number one lower than the one on screen is a trap.
    pub(super) page: usize,
}

/// Parse `view`'s arguments. `Err` carries the message to print on stderr.
pub(super) fn parse_view_args(args: &[String]) -> Result<ViewArgs, String> {
    let mut path: Option<PathBuf> = None;
    let mut page: Option<usize> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--page" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--page needs a page number".to_owned())?;
                let parsed: usize = value
                    .parse()
                    .map_err(|_| format!("--page expects a number, got {value:?}"))?;
                if parsed == 0 {
                    return Err("--page is one-based, so 0 is not a page".to_owned());
                }
                page = Some(parsed - 1);
                index += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown option {other:?}"));
            }
            other => {
                if path.is_some() {
                    return Err("view takes exactly one path".to_owned());
                }
                path = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }

    Ok(ViewArgs {
        path: path.ok_or_else(|| "view needs a path".to_owned())?,
        page: page.unwrap_or(0),
    })
}

pub(super) fn run_view_command(args: &[String]) -> std::io::Result<i32> {
    let parsed = match parse_view_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("herdr view: {message}");
            eprintln!("usage: herdr view <PATH> [--page N]");
            return Ok(2);
        }
    };

    // Checked before the terminal is taken over: a message printed after
    // entering the alternate screen is wiped by leaving it, so the reader would
    // see a tab flash and close with nothing said.
    if !parsed.path.is_file() {
        eprintln!(
            "herdr view: {} is not a readable file",
            parsed.path.display()
        );
        return Ok(1);
    }

    crate::viewer::run(&parsed.path, parsed.page)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    // TP-FVIEW-TAB-13: a bare path is enough, and pages start at the first one.
    #[test]
    fn a_bare_path_opens_the_first_page() {
        let parsed = parse_view_args(&args(&["/tmp/manual.pdf"])).expect("a bare path is valid");
        assert_eq!(parsed.path, PathBuf::from("/tmp/manual.pdf"));
        assert_eq!(parsed.page, 0);
    }

    // TP-FVIEW-TAB-14: `--page` is one-based on the way in and zero-based on
    // the way out. The viewer prints "page 3 of 10"; asking the reader to type
    // 2 for that page would be a trap, and mixing the conventions anywhere but
    // this one line is how page navigation goes wrong.
    #[test]
    fn the_page_option_is_one_based_for_the_reader() {
        let parsed = parse_view_args(&args(&["/tmp/a.pdf", "--page", "3"])).expect("valid");
        assert_eq!(parsed.page, 2);

        let zero = parse_view_args(&args(&["/tmp/a.pdf", "--page", "0"]));
        assert!(zero.is_err(), "0 is not a page a reader can see");
    }

    // TP-FVIEW-TAB-15: malformed input is a message, never a panic. This runs
    // inside a pane; a panic there prints a backtrace nobody asked for and the
    // tab closes on it.
    #[test]
    fn malformed_arguments_are_refused_with_a_message() {
        for bad in [
            vec!["--page", "3"],
            vec!["/tmp/a.pdf", "--page"],
            vec!["/tmp/a.pdf", "--page", "many"],
            vec!["/tmp/a.pdf", "/tmp/b.pdf"],
            vec!["/tmp/a.pdf", "--zoom"],
            vec![],
        ] {
            let result = parse_view_args(&args(&bad));
            assert!(result.is_err(), "{bad:?} should be refused");
            assert!(
                !result.unwrap_err().is_empty(),
                "{bad:?} must say why it was refused"
            );
        }
    }
}
