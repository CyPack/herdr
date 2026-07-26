//! Sending a file to another machine on the tailnet.
//!
//! Split the same way the rest of herdr is: everything that decides something
//! is pure and lives here with its tests, and the only impure part is running
//! `tailscale` and handing its bytes back. The device list, the labels, the
//! ordering and the exact command line are all computed from data, so none of
//! it needs a tailnet to test.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One machine on the tailnet, as the picker needs to show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TailscaleDevice {
    /// What to show. Usually the host name; see [`parse_devices`] for why it is
    /// not always.
    pub(crate) label: String,
    /// The name to hand `tailscale file cp`. Unique across the tailnet, which
    /// the host name is not.
    pub(crate) target: String,
    pub(crate) os: String,
    pub(crate) online: bool,
}

/// Read `tailscale status --json` into the device list.
///
/// Three decisions live here rather than in the UI:
///
/// * **Self is dropped.** Taildrop refuses a send to the machine it starts on,
///   so offering it would be a menu entry that can only fail.
/// * **The label is the host name until host names collide.** This tailnet has
///   three machines called `localhost` and two called `fedora`; a picker where
///   two rows read the same gives the reader no way to choose. When a name is
///   shared, every device carrying it falls back to the first label of its DNS
///   name, which is unique by construction.
/// * **Online first, then by label.** The devices that can receive right now
///   are the ones being looked for; alphabetical order alone buries them.
pub(crate) fn parse_devices(json: &str) -> Result<Vec<TailscaleDevice>, String> {
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("tailscale status: {error}"))?;
    // Valid JSON that is not a status report reaches here whenever something
    // other than tailscale answered — a wrapper script, an error object, a
    // different binary on PATH. Treating it as an empty tailnet would show an
    // empty picker and blame the network.
    if !root.is_object() {
        return Err("tailscale status: expected an object".to_owned());
    }

    // A tailnet of one has no `Peer` key at all. That is an empty list, not a
    // failure: the picker can say "no other devices" but an error would send
    // the reader looking for a fault that is not there.
    let peers = match root.get("Peer") {
        None | Some(serde_json::Value::Null) => return Ok(Vec::new()),
        Some(serde_json::Value::Object(peers)) => peers,
        Some(_) => return Err("tailscale status: Peer is not an object".to_owned()),
    };

    let mut raw: Vec<(String, String, String, bool)> = Vec::new();
    for peer in peers.values() {
        let host = peer
            .get("HostName")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let dns = peer
            .get("DNSName")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim_end_matches('.')
            .to_owned();
        // Without a name there is nothing to send to and nothing to show.
        if host.is_empty() && dns.is_empty() {
            continue;
        }
        let os = peer
            .get("OS")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let online = peer
            .get("Online")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        raw.push((host, dns, os, online));
    }

    let mut host_counts: HashMap<&str, usize> = HashMap::new();
    for (host, _, _, _) in &raw {
        *host_counts.entry(host.as_str()).or_insert(0) += 1;
    }

    let mut devices: Vec<TailscaleDevice> = raw
        .iter()
        .map(|(host, dns, os, online)| {
            let dns_label = dns.split('.').next().unwrap_or_default();
            let shared =
                host.is_empty() || host_counts.get(host.as_str()).copied().unwrap_or(0) > 1;
            let label = if shared && !dns_label.is_empty() {
                dns_label.to_owned()
            } else {
                host.clone()
            };
            TailscaleDevice {
                label,
                // `tailscale file cp` takes either name; the DNS name is the one
                // that is unique, so it is the one that gets sent.
                target: if dns.is_empty() {
                    host.clone()
                } else {
                    dns.clone()
                },
                os: os.clone(),
                online: *online,
            }
        })
        .collect();

    sort_devices(&mut devices, &[]);
    Ok(devices)
}

/// Order the list the picker shows.
///
/// Three tiers, in this order: pinned machines, then whatever is online, then
/// the rest alphabetically.
///
/// Pinned beats online deliberately. A pin is a standing statement — *this is
/// the machine I send to* — and a laptop that happens to be asleep right now
/// should not drop below fourteen others the reader has never sent anything to.
/// Taildrop queues for it either way.
///
/// `pinned` is ordered: the first entry sorts above the second, so the reader's
/// own ordering is preserved rather than re-sorted alphabetically underneath
/// them.
pub(crate) fn sort_devices(devices: &mut [TailscaleDevice], pinned: &[String]) {
    let rank = |device: &TailscaleDevice| -> usize {
        pinned
            .iter()
            .position(|target| target == &device.target)
            .unwrap_or(usize::MAX)
    };
    devices.sort_by(|left, right| {
        rank(left)
            .cmp(&rank(right))
            .then_with(|| right.online.cmp(&left.online))
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
            .then_with(|| left.target.cmp(&right.target))
    });
}

/// Add or remove `target` from `pinned`, answering with the new list.
///
/// A newly pinned machine goes to the end rather than the front: pinning a
/// second machine must not push the first one down, or the reader's top slot
/// changes every time they pin something.
pub(crate) fn toggle_pin(pinned: &[String], target: &str) -> Vec<String> {
    let mut next: Vec<String> = pinned.to_vec();
    match next.iter().position(|entry| entry == target) {
        Some(index) => {
            next.remove(index);
        }
        None => next.push(target.to_owned()),
    }
    next
}

/// The command that sends `paths` to `target`.
///
/// Returned rather than run so the exact argument list is testable. `--` is not
/// decoration: a file named `-n` is an ordinary file, and without the separator
/// `tailscale` would read it as a flag and fail on something the reader picked
/// in a file manager.
pub(crate) fn send_command(paths: &[PathBuf], target: &str) -> Vec<String> {
    let mut command = vec![
        "tailscale".to_owned(),
        "file".to_owned(),
        "cp".to_owned(),
        "--".to_owned(),
    ];
    command.extend(paths.iter().map(|path| path.to_string_lossy().into_owned()));
    command.push(format!("{target}:"));
    command
}

/// What to put on the picker's status line once a send has finished.
///
/// The file names are named back to the reader. "Sent" alone leaves them
/// checking whether the thing they meant to send is the thing that went.
pub(crate) fn send_outcome(paths: &[PathBuf], device_label: &str, error: Option<&str>) -> String {
    let names: Vec<String> = paths
        .iter()
        .map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        })
        .collect();
    let what = match names.len() {
        0 => "nothing".to_owned(),
        1 => names[0].clone(),
        count => format!("{count} files"),
    };
    match error {
        None => format!("sent {what} to {device_label}"),
        Some(reason) if reason.trim().is_empty() => {
            format!("could not send {what} to {device_label}")
        }
        Some(reason) => format!(
            "could not send {what} to {device_label}: {}",
            reason.trim().lines().next().unwrap_or_default()
        ),
    }
}

/// Whether a path can be handed to Taildrop at all.
///
/// Taildrop takes files. A directory silently sends nothing, so it is refused
/// here with a reason rather than failing later with tailscale's own wording.
pub(crate) fn unsendable_reason(path: &Path) -> Option<String> {
    if path.is_dir() {
        return Some(format!(
            "{} is a folder, and Taildrop only takes files",
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        ));
    }
    None
}

/// How long either `tailscale` call is given before it is given up on.
///
/// Measured on this machine: `status --json` answers in 10-40 ms. The limit is
/// not there for the normal case but for the one where `tailscaled` is not
/// answering at all — without it, a menu click would freeze the whole UI for as
/// long as the daemon stayed silent.
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// Run `program args...`, giving up after [`DEADLINE`].
///
/// Returns stdout on success, and on failure a message already fit to show:
/// stderr if the tool said anything, its own words otherwise.
fn run_with_deadline(command: &[String]) -> Result<String, String> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| "empty command".to_owned())?;

    let mut child = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            // The most likely failure by far, and the one with the most useful
            // answer: tailscale simply is not installed.
            std::io::ErrorKind::NotFound => format!("{program} is not installed"),
            _ => format!("{program}: {error}"),
        })?;

    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() >= DEADLINE {
                    // Best effort: if the kill fails the child is already gone,
                    // which is the outcome being asked for anyway.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{program} did not answer"));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => return Err(format!("{program}: {error}")),
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("{program}: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if stderr.is_empty() {
        format!("{program} failed")
    } else {
        stderr
    })
}

/// Ask the local tailscaled which machines are on the tailnet.
pub(crate) fn load_devices() -> Result<Vec<TailscaleDevice>, String> {
    let command = [
        "tailscale".to_owned(),
        "status".to_owned(),
        "--json".to_owned(),
    ];
    parse_devices(&run_with_deadline(&command)?)
}

/// Hand `paths` to Taildrop for `target`.
pub(crate) fn send(paths: &[PathBuf], target: &str) -> Result<(), String> {
    if paths.is_empty() {
        return Err("nothing selected".to_owned());
    }
    for path in paths {
        if let Some(reason) = unsendable_reason(path) {
            return Err(reason);
        }
    }
    run_with_deadline(&send_command(paths, target)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped after real `tailscale status --json` output, including this
    /// tailnet's actual duplicate host names.
    fn status_json() -> String {
        serde_json::json!({
            "Self": {
                "HostName": "fedora",
                "DNSName": "mainfedora.tailb1cc10.ts.net.",
                "OS": "linux",
                "Online": true
            },
            "Peer": {
                "key-a": {
                    "HostName": "asus-X550CA",
                    "DNSName": "asus-x550ca.tailb1cc10.ts.net.",
                    "OS": "linux",
                    "Online": false
                },
                "key-b": {
                    "HostName": "fedora",
                    "DNSName": "fedora-2.tailb1cc10.ts.net.",
                    "OS": "linux",
                    "Online": true
                },
                "key-c": {
                    "HostName": "fedora",
                    "DNSName": "fedora-3.tailb1cc10.ts.net.",
                    "OS": "linux",
                    "Online": false
                },
                "key-d": {
                    "HostName": "macbook",
                    "DNSName": "macbook.tailb1cc10.ts.net.",
                    "OS": "macOS",
                    "Online": true
                }
            }
        })
        .to_string()
    }

    // TP-FSEND-TS-01: the machine herdr is running on never appears. Taildrop
    // refuses a send to itself, so listing it offers a choice that can only
    // fail.
    #[test]
    fn the_local_machine_is_not_a_destination() {
        let devices = parse_devices(&status_json()).expect("valid status");
        assert!(
            !devices
                .iter()
                .any(|d| d.target == "mainfedora.tailb1cc10.ts.net"),
            "self must not be listed: {devices:?}"
        );
        assert_eq!(devices.len(), 4);
    }

    // TP-FSEND-TS-02: devices that can receive right now come first, and the
    // rest stay alphabetical. Sorting by name alone buries the two machines
    // that are actually reachable under thirteen that are not.
    #[test]
    fn reachable_devices_are_listed_first() {
        let devices = parse_devices(&status_json()).expect("valid status");
        let online: Vec<bool> = devices.iter().map(|d| d.online).collect();
        assert_eq!(online, vec![true, true, false, false], "{devices:?}");
        assert_eq!(devices[0].label, "fedora-2");
        assert_eq!(devices[1].label, "macbook");
    }

    // TP-FSEND-TS-03: when two machines share a host name, both fall back to
    // their DNS label. Two rows reading `fedora` give the reader no way to tell
    // which is which, and picking the wrong one sends the file to the wrong
    // computer.
    #[test]
    fn duplicate_host_names_are_disambiguated() {
        let devices = parse_devices(&status_json()).expect("valid status");
        let labels: Vec<&str> = devices.iter().map(|d| d.label.as_str()).collect();
        assert!(
            !labels.contains(&"fedora"),
            "the shared name must not be shown as-is: {labels:?}"
        );
        assert!(labels.contains(&"fedora-2") && labels.contains(&"fedora-3"));
        // A name nobody else uses is left alone.
        assert!(labels.contains(&"macbook"));
    }

    // TP-FSEND-TS-04: the send target is the DNS name, which is unique, not the
    // host name, which is not. Sending to `fedora` on this tailnet is ambiguous.
    #[test]
    fn the_send_target_is_the_unique_name() {
        let devices = parse_devices(&status_json()).expect("valid status");
        let fedora_2 = devices
            .iter()
            .find(|d| d.label == "fedora-2")
            .expect("fedora-2 is listed");
        assert_eq!(fedora_2.target, "fedora-2.tailb1cc10.ts.net");
        assert!(
            !fedora_2.target.ends_with('.'),
            "the trailing dot is not part of the name a user types"
        );
    }

    // TP-FSEND-TS-19: a pinned machine sorts above everything, including the
    // ones that are online. A pin is a standing statement about where the
    // reader sends; a sleeping laptop that drops below fourteen strangers makes
    // the pin worthless, and Taildrop queues for it either way.
    #[test]
    fn a_pinned_device_outranks_an_online_one() {
        let mut devices = parse_devices(&status_json()).expect("valid status");
        // asus-X550CA is offline and would otherwise sort third.
        sort_devices(&mut devices, &["asus-x550ca.tailb1cc10.ts.net".to_owned()]);
        assert_eq!(devices[0].label, "asus-X550CA", "{devices:?}");
        assert!(!devices[0].online, "the pin held despite being offline");
        // Everything below the pin keeps the old order.
        assert!(devices[1].online && devices[2].online);
    }

    // TP-FSEND-TS-20: the reader's own pin order is preserved rather than
    // re-sorted. Pinning two machines and having them swap alphabetically
    // defeats the point of choosing an order.
    #[test]
    fn pins_keep_the_order_they_were_added_in() {
        let mut devices = parse_devices(&status_json()).expect("valid status");
        let pinned = vec![
            "macbook.tailb1cc10.ts.net".to_owned(),
            "asus-x550ca.tailb1cc10.ts.net".to_owned(),
        ];
        sort_devices(&mut devices, &pinned);
        assert_eq!(devices[0].label, "macbook");
        assert_eq!(devices[1].label, "asus-X550CA");
    }

    // TP-FSEND-TS-21: pinning is a toggle, and a new pin lands at the end.
    // Pushing it to the front would move the reader's top slot every time they
    // pinned anything.
    #[test]
    fn pinning_toggles_and_appends() {
        let pinned = toggle_pin(&[], "a.ts.net");
        assert_eq!(pinned, vec!["a.ts.net"]);
        let pinned = toggle_pin(&pinned, "b.ts.net");
        assert_eq!(pinned, vec!["a.ts.net", "b.ts.net"]);
        let pinned = toggle_pin(&pinned, "a.ts.net");
        assert_eq!(pinned, vec!["b.ts.net"], "a second press unpins");
    }

    // TP-FSEND-TS-05: a tailnet of one is an empty list, not a failure. An
    // error here would send the reader looking for a fault that is not there.
    #[test]
    fn a_tailnet_of_one_is_empty_not_broken() {
        let alone = serde_json::json!({ "Self": { "HostName": "fedora" } }).to_string();
        assert_eq!(parse_devices(&alone).expect("valid"), Vec::new());
    }

    // TP-FSEND-TS-06: unreadable output is a message, never a panic. This runs
    // when the reader opens a menu; a panic there takes herdr down with it.
    #[test]
    fn unreadable_status_is_refused_with_a_message() {
        for bad in ["", "not json", "[]", r#"{"Peer": 7}"#] {
            let result = parse_devices(bad);
            assert!(result.is_err(), "{bad:?} should be refused");
            assert!(!result.unwrap_err().is_empty(), "{bad:?} must say why");
        }
    }

    // TP-FSEND-TS-07: the command separates options from file names. A file
    // called `-n` is an ordinary file a reader can select, and without `--`
    // tailscale reads it as a flag and the send fails on something that was
    // never a flag.
    #[test]
    fn file_names_cannot_be_read_as_options() {
        let command = send_command(
            &[PathBuf::from("/home/a/-n"), PathBuf::from("/home/a/b.pdf")],
            "box.ts.net",
        );
        assert_eq!(
            command,
            vec![
                "tailscale",
                "file",
                "cp",
                "--",
                "/home/a/-n",
                "/home/a/b.pdf",
                "box.ts.net:"
            ]
        );
    }

    // TP-FSEND-TS-08: the outcome names the files back. "Sent" alone leaves the
    // reader checking whether what went is what they meant.
    #[test]
    fn the_outcome_names_what_was_sent() {
        let one = [PathBuf::from("/home/a/report.pdf")];
        assert_eq!(
            send_outcome(&one, "macbook", None),
            "sent report.pdf to macbook"
        );

        let many = [PathBuf::from("/a/x.png"), PathBuf::from("/a/y.png")];
        assert_eq!(
            send_outcome(&many, "macbook", None),
            "sent 2 files to macbook"
        );

        let failed = send_outcome(&one, "macbook", Some("no such host\nsecond line"));
        assert!(
            failed.starts_with("could not send report.pdf to macbook:"),
            "{failed}"
        );
        assert!(
            !failed.contains("second line"),
            "a status line is one line: {failed}"
        );
    }

    // TP-FSEND-TS-09: a folder is refused before anything is spawned, in
    // herdr's own words. Taildrop takes files; handed a directory it reports
    // nothing useful and the reader is left thinking the send worked.
    #[test]
    fn folders_are_refused_with_a_reason() {
        let dir = std::env::temp_dir();
        let reason = unsendable_reason(&dir).expect("a folder cannot be sent");
        assert!(reason.contains("folder"), "{reason}");
        assert_eq!(
            unsendable_reason(Path::new("/definitely/not/here.png")),
            None
        );
    }
}
