//! `herdr chat ...` — seat chats under drawers/modules over the socket API.
//!
//! The single-chat road is the TUI's "Move to branch/module..." menu; this
//! CLI exists for the BULK road: a seat plan produced by an analyzer
//! (scripts/chat_context.py) is applied in one request, stamped with its
//! source so it can never override a person's own menu moves and can be
//! withdrawn wholesale (TP-CHAT-MOVE-13/14).

use crate::api::schema::{ChatSeatEntry, ChatSeatParams, ChatUnseatParams};

/// The stamp a plan application wears unless the plan or the flag says
/// otherwise — matches the analyzer's default output.
const DEFAULT_PLAN_SOURCE: &str = "seat-plan";

pub(super) fn run_chat_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_chat_help();
        return Ok(2);
    };

    match subcommand {
        "seat" => chat_seat(&args[1..]),
        "move" => chat_move(&args[1..]),
        "unseat" => chat_unseat(&args[1..]),
        "help" | "--help" | "-h" => {
            print_chat_help();
            Ok(0)
        }
        _ => {
            print_chat_help();
            Ok(2)
        }
    }
}

fn chat_seat(args: &[String]) -> std::io::Result<i32> {
    let mut plan_path = None;
    let mut source = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--plan" => {
                index += 1;
                plan_path = args.get(index).cloned();
            }
            "--source" => {
                index += 1;
                source = args.get(index).cloned();
            }
            _ => {
                eprintln!("usage: herdr chat seat --plan <FILE> [--source NAME]");
                return Ok(2);
            }
        }
        index += 1;
    }
    let Some(plan_path) = plan_path else {
        eprintln!("usage: herdr chat seat --plan <FILE> [--source NAME]");
        return Ok(2);
    };

    let raw = std::fs::read_to_string(&plan_path)?;
    let plan: serde_json::Value = serde_json::from_str(&raw).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("plan is not valid JSON ({plan_path}): {err}"),
        )
    })?;
    let Some(rows) = plan.get("seats").and_then(|v| v.as_array()) else {
        eprintln!("plan has no \"seats\" array: {plan_path}");
        return Ok(2);
    };
    let mut seats = Vec::with_capacity(rows.len());
    for row in rows {
        let sid = row.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
        let key = row.get("target_key").and_then(|v| v.as_str()).unwrap_or("");
        if sid.is_empty() || key.is_empty() {
            eprintln!("plan row without session_id/target_key is skipped");
            continue;
        }
        let extra_keys = row
            .get("extra_keys")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        seats.push(ChatSeatEntry {
            session_id: sid.to_string(),
            target_key: key.to_string(),
            extra_keys,
        });
    }
    if seats.is_empty() {
        eprintln!("plan carries no applicable seats: {plan_path}");
        return Ok(2);
    }
    let source = source
        .or_else(|| {
            plan.get("source")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| DEFAULT_PLAN_SOURCE.to_string());

    super::runtime::chat_seat(ChatSeatParams {
        seats,
        source: Some(source),
    })
}

fn chat_move(args: &[String]) -> std::io::Result<i32> {
    let mut session_id = None;
    let mut target = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--to" => {
                index += 1;
                target = args.get(index).cloned();
            }
            arg if session_id.is_none() && !arg.starts_with('-') => {
                session_id = Some(arg.to_string());
            }
            _ => {
                eprintln!("usage: herdr chat move <SESSION_ID> --to <LEDGER_KEY>");
                return Ok(2);
            }
        }
        index += 1;
    }
    let (Some(session_id), Some(target)) = (session_id, target) else {
        eprintln!("usage: herdr chat move <SESSION_ID> --to <LEDGER_KEY>");
        return Ok(2);
    };
    // A person typing the command IS the user road — same stamp as the menu.
    super::runtime::chat_seat(ChatSeatParams {
        seats: vec![ChatSeatEntry {
            session_id,
            target_key: target,
            extra_keys: Vec::new(),
        }],
        source: None,
    })
}

fn chat_unseat(args: &[String]) -> std::io::Result<i32> {
    let mut source = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source" => {
                index += 1;
                source = args.get(index).cloned();
            }
            _ => {
                eprintln!("usage: herdr chat unseat --source <NAME>");
                return Ok(2);
            }
        }
        index += 1;
    }
    let Some(source) = source else {
        eprintln!("usage: herdr chat unseat --source <NAME>");
        return Ok(2);
    };
    super::runtime::chat_unseat(ChatUnseatParams { source })
}

fn print_chat_help() {
    println!("usage: herdr chat <command>");
    println!();
    println!("Seat agent chats under branch drawers or module seats");
    println!();
    println!("Commands:");
    println!("  seat --plan <FILE> [--source NAME]   Apply a bulk seat plan (JSON with a \"seats\" array)");
    println!(
        "  move <SESSION_ID> --to <LEDGER_KEY>  Move one chat (a checkout dir or module:<key>)"
    );
    println!("  unseat --source <NAME>               Withdraw every move that source wrote");
}
