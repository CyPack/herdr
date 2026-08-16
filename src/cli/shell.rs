//! `herdr shell spec [--json]` — what a bar config may say.
//!
//! A thin mouth for [`crate::ui::shell::shell_spec`]. Everything the command
//! prints is derived from the tables the parser itself dispatches on, so this
//! file holds no grammar of its own and cannot drift away from one.
//!
//! Deliberately server-free. The question "what may I write?" comes before
//! anything is running, and an answer that needed a live session would be
//! unavailable exactly when it is most wanted.

/// What `shell spec` was asked to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpecFormat {
    Text,
    Json,
}

/// Parse `spec`'s arguments. `Err` carries the message to print on stderr.
pub(super) fn parse_spec_args(args: &[String]) -> Result<SpecFormat, String> {
    let mut format = SpecFormat::Text;

    for argument in args {
        match argument.as_str() {
            "--json" => format = SpecFormat::Json,
            "--format" => {
                return Err("--format is not a spec option; use --json".to_owned());
            }
            other => return Err(format!("unknown option {other:?}")),
        }
    }

    Ok(format)
}

pub(super) fn run_shell_command(args: &[String]) -> std::io::Result<i32> {
    match args.first().map(|arg| arg.as_str()) {
        Some("spec") => run_spec_command(&args[1..]),
        Some("help" | "--help" | "-h") | None => {
            print_shell_help();
            Ok(0)
        }
        Some(other) => {
            eprintln!("unknown shell command {other:?}");
            print_shell_help();
            Ok(2)
        }
    }
}

fn run_spec_command(args: &[String]) -> std::io::Result<i32> {
    let format = match parse_spec_args(args) {
        Ok(format) => format,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("usage: herdr shell spec [--json]");
            return Ok(2);
        }
    };

    let spec = crate::ui::shell::shell_spec();
    match format {
        SpecFormat::Text => print!("{}", crate::ui::shell::render_shell_spec_text(&spec)),
        SpecFormat::Json => match serde_json::to_string_pretty(&spec) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                // Serialising a struct of owned strings and numbers has no
                // failing path today. Said rather than unwrapped, because the
                // day it grows one, a person deserves the reason instead of a
                // stack trace.
                eprintln!("the shell spec could not be written as JSON: {err}");
                return Ok(1);
            }
        },
    }
    Ok(0)
}

fn print_shell_help() {
    println!("usage: herdr shell spec [--json]");
    println!();
    println!("  spec    Print the bar grammar: the kinds, keys and examples a");
    println!("          config may use. --json for a machine to read.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// TP-SPEC-11: the format is text unless somebody asks otherwise.
    ///
    /// Text is the default because a person typing the command is reading it,
    /// and a wall of JSON is the wrong answer to a question asked by hand.
    #[test]
    fn the_spec_prints_text_until_json_is_asked_for() {
        assert_eq!(parse_spec_args(&args(&[])), Ok(SpecFormat::Text));
        assert_eq!(parse_spec_args(&args(&["--json"])), Ok(SpecFormat::Json));
    }

    /// TP-SPEC-11: an option this command does not have is refused by name.
    ///
    /// Silently ignoring it would print the default and let somebody believe
    /// they had asked for something else — the failure that costs the most to
    /// notice, because the output looks fine.
    #[test]
    fn an_option_the_spec_does_not_have_is_refused_rather_than_ignored() {
        assert!(parse_spec_args(&args(&["--yaml"])).is_err());
        assert!(parse_spec_args(&args(&["--json", "--yaml"])).is_err());
        // `--format text|json` is the shape other commands use, so somebody will
        // try it here. Saying which option this one wants costs one line and
        // saves a search.
        let refusal = parse_spec_args(&args(&["--format"])).expect_err("--format is not an option");
        assert!(refusal.contains("--json"), "{refusal}");
    }
}
