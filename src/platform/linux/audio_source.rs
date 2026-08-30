//! Which PipeWire streams can be captured, and which process really made them.
//!
//! Reading the graph is the easy half. Naming the owner is the half that was
//! measured wrong twice, so the rule here is written from the graph as it
//! actually looked on a live desktop rather than from the property names' own
//! promises.
//!
//! `pipewire.sec.pid` is the kernel-verified pid of the *connection*, which
//! makes it the honest answer only when the program connected to PipeWire
//! itself. Everything that speaks the Pulse protocol reaches the graph through
//! one bridge, and every one of those connections carries the bridge's pid. On
//! the measured desktop seven of twelve clients — a browser, a second browser,
//! a speech daemon, two volume controls, a sound-effect library — all reported
//! the same verified pid, which belongs to none of them. A rule that trusts
//! that field alone attributes every ordinary application's sound to one
//! process whose ancestry reaches no pane at all, and so matches nothing,
//! forever, without ever looking wrong.
//!
//! The bridge is found from the graph itself instead of from a list of names:
//! a pid that fronts two or more clients is multiplexing, not producing. When
//! the verified pid turns out to be such a pid, the owner is taken from the
//! self-reported `application.process.id` and marked as self-reported. That is
//! safe here because the only thing done with a pid afterwards is an ancestry
//! walk through `/proc`: a lie would have to name a process that already sits
//! inside the pane's own tree, which is not a lie worth telling.
//!
//! TP-MEDIA-GRAPH-01.

// Unused until the supervisor that watches the graph lands: the reader is
// built and pinned first so the driver has something already tested to call.
//
// REMOVAL CONDITION: delete this attribute the moment `parse_output_streams`
// is called from the pane-audio supervisor — after that, a dead item here is a
// real leak, not a staged one.
#![allow(dead_code)]

use std::collections::HashMap;

use serde_json::Value;

/// How much the pid can be trusted, which is worth carrying because the two
/// answers come from different places and deserve different log lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PidTrust {
    /// The kernel told PipeWire who connected.
    Verified,
    /// The program told PipeWire who it is.
    SelfReported,
}

/// One capturable output stream, as the graph describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawStream {
    pub(crate) node_id: u32,
    pub(crate) pid: Option<u32>,
    pub(crate) pid_trust: Option<PidTrust>,
    pub(crate) app_name: Option<String>,
    /// What `pw-record --target` wants. The graph's object id and this serial
    /// are different numbers, and targeting with the wrong one captures the
    /// wrong stream — quietly, because both are valid ids for something.
    pub(crate) object_serial: Option<u32>,
}

const AUDIO_OUTPUT_CLASS: &str = "Stream/Output/Audio";

/// Reads a `pw-dump` document into the output streams it describes.
pub(crate) fn parse_output_streams(dump: &str) -> Result<Vec<RawStream>, serde_json::Error> {
    let objects: Vec<Value> = serde_json::from_str(dump)?;

    // Clients first: a node points at the client that opened it, and the
    // owner's identity lives there rather than on the node.
    let mut clients: HashMap<u32, &Value> = HashMap::new();
    let mut fronted: HashMap<u32, usize> = HashMap::new();
    for object in &objects {
        if !is_interface(object, "Client") {
            continue;
        }
        let (Some(id), Some(props)) = (object_id(object), object_props(object)) else {
            continue;
        };
        clients.insert(id, props);
        if let Some(pid) = prop_u32(Some(props), "pipewire.sec.pid") {
            *fronted.entry(pid).or_default() += 1;
        }
    }

    let mut streams = Vec::new();
    for object in &objects {
        if !is_interface(object, "Node") {
            continue;
        }
        let (Some(node_id), Some(props)) = (object_id(object), object_props(object)) else {
            continue;
        };
        if prop_str(Some(props), "media.class") != Some(AUDIO_OUTPUT_CLASS) {
            continue;
        }
        let client = prop_u32(Some(props), "client.id")
            .and_then(|client_id| clients.get(&client_id).copied());
        let (pid, pid_trust) = resolve_owner(props, client, &fronted);
        let app_name = prop_str(Some(props), "application.name")
            .or_else(|| prop_str(Some(props), "node.name"))
            .or_else(|| prop_str(client, "application.name"))
            .map(str::to_owned);
        streams.push(RawStream {
            node_id,
            pid,
            pid_trust,
            app_name,
            object_serial: None,
        });
    }
    Ok(streams)
}

/// Names the process behind a stream, preferring what the kernel saw over what
/// the program says — except where the kernel saw a multiplexer.
fn resolve_owner(
    node: &Value,
    client: Option<&Value>,
    fronted: &HashMap<u32, usize>,
) -> (Option<u32>, Option<PidTrust>) {
    for props in [Some(node), client].into_iter().flatten() {
        if let Some(pid) = prop_u32(Some(props), "pipewire.sec.pid") {
            if !is_multiplexer(pid, fronted) {
                return (Some(pid), Some(PidTrust::Verified));
            }
        }
    }
    for props in [Some(node), client].into_iter().flatten() {
        if let Some(pid) = prop_u32(Some(props), "application.process.id") {
            return (Some(pid), Some(PidTrust::SelfReported));
        }
    }
    (None, None)
}

/// A pid that fronts more than one client is forwarding other programs' sound,
/// so it names the bridge rather than the producer. Read from the graph, not
/// from a list of process names, because the list would be wrong on the first
/// desktop that bridges through something else.
fn is_multiplexer(pid: u32, fronted: &HashMap<u32, usize>) -> bool {
    fronted.get(&pid).is_some_and(|clients| *clients > 1)
}

fn is_interface(object: &Value, suffix: &str) -> bool {
    object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.ends_with(suffix))
}

fn object_id(object: &Value) -> Option<u32> {
    prop_u32(Some(object), "id")
}

fn object_props(object: &Value) -> Option<&Value> {
    object.get("info")?.get("props")
}

/// Props survive several PipeWire versions in which the same key is a number
/// in one and a string in the next, so both are read.
fn prop_u32(props: Option<&Value>, key: &str) -> Option<u32> {
    let value = props?.get(key)?;
    match value {
        Value::Number(number) => number.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn prop_str<'a>(props: Option<&'a Value>, key: &str) -> Option<&'a str> {
    props?.get(key)?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of a real `pw-dump`, with the identifying content replaced.
    /// Two clients front the same verified pid (the bridge), one connects on
    /// its own, one stream names no process at all, and one node is an input
    /// rather than an output.
    const DUMP: &str = r#"[
      {"id": 34, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 900, "application.name": "pipewire"}}},
      {"id": 118, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 900, "application.process.id": 4711,
                          "application.name": "Browser"}}},
      {"id": 119, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 900, "application.process.id": 4712,
                          "application.name": "Reader"}}},
      {"id": 70, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 5150, "application.name": "Shell"}}},
      {"id": 200, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Output/Audio", "client.id": 118,
                          "object.serial": 2374,
                          "application.name": "Browser", "media.name": "playback"}}},
      {"id": 201, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Output/Audio", "client.id": 70,
                          "application.name": "Shell"}}},
      {"id": 202, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Input/Audio", "client.id": 118,
                          "application.name": "Browser"}}},
      {"id": 203, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Output/Audio", "node.name": "orphan"}}}
    ]"#;

    fn stream(streams: &[RawStream], node_id: u32) -> &RawStream {
        streams
            .iter()
            .find(|stream| stream.node_id == node_id)
            .unwrap_or_else(|| panic!("node {node_id} missing from {streams:?}"))
    }

    /// PW-1 — the measured failure: a bridged client's verified pid belongs to
    /// the bridge, so the owner has to come from what the program says about
    /// itself.
    #[test]
    fn a_bridged_stream_resolves_to_the_program_not_the_bridge() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        let browser = stream(&streams, 200);
        assert_eq!(browser.pid, Some(4711));
        assert_eq!(browser.pid_trust, Some(PidTrust::SelfReported));
    }

    /// PW-2 — a program that connected on its own keeps its verified pid.
    #[test]
    fn a_direct_client_keeps_its_verified_pid() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        let shell = stream(&streams, 201);
        assert_eq!(shell.pid, Some(5150));
        assert_eq!(shell.pid_trust, Some(PidTrust::Verified));
    }

    /// PW-3 — capture is about what leaves the machine; an input stream is a
    /// microphone and has no business here.
    #[test]
    fn input_streams_are_not_capture_candidates() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        assert!(streams.iter().all(|stream| stream.node_id != 202));
    }

    /// PW-4 — a stream nobody claims still gets listed: the name rule may yet
    /// recognise it, and dropping it here would hide it from every later rule.
    #[test]
    fn a_stream_without_any_pid_is_still_listed() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        let orphan = stream(&streams, 203);
        assert_eq!(orphan.pid, None);
        assert_eq!(orphan.pid_trust, None);
        assert_eq!(orphan.app_name.as_deref(), Some("orphan"));
    }

    /// PW-5 — the browser's name travels with it, because the weakest match
    /// rule has nothing else to work with.
    #[test]
    fn the_producer_name_survives_the_read() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        assert_eq!(stream(&streams, 200).app_name.as_deref(), Some("Browser"));
    }

    /// PW-9 — the capture target is the stream's serial, not the graph's
    /// object id: `pw-record --target` reads the serial, and the two numbers
    /// differ, so carrying only the object id would aim the recorder at
    /// whatever else happens to hold that number.
    #[test]
    fn the_capture_target_is_the_streams_serial() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        assert_eq!(stream(&streams, 200).object_serial, Some(2374));
        assert_eq!(stream(&streams, 203).object_serial, None);
    }

    /// PW-6 — an unreadable graph is an error to report, never a panic in a
    /// server loop, and never an empty list that reads as silence.
    #[test]
    fn a_malformed_dump_is_an_error_not_a_panic() {
        assert!(parse_output_streams("{not json").is_err());
    }

    /// PW-7 — an empty graph is silence, which is legal.
    #[test]
    fn an_empty_graph_is_no_streams() {
        assert_eq!(
            parse_output_streams("[]").expect("empty parses"),
            Vec::new()
        );
    }

    /// PW-8 — versions differ on whether a prop is a number or a string; both
    /// have to read the same.
    #[test]
    fn props_are_read_as_numbers_or_strings() {
        let dump = r#"[
          {"id": 5, "type": "PipeWire:Interface:Client",
           "info": {"props": {"pipewire.sec.pid": "77", "application.name": "Text"}}},
          {"id": 6, "type": "PipeWire:Interface:Node",
           "info": {"props": {"media.class": "Stream/Output/Audio", "client.id": "5"}}}
        ]"#;
        let streams = parse_output_streams(dump).expect("dump parses");
        assert_eq!(stream(&streams, 6).pid, Some(77));
    }
}
