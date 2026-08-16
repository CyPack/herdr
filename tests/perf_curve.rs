//! Render cost curve: what does a client cost, and where does it bend?
//!
//! Measurement harness, not a regression test — run explicitly:
//!
//! ```bash
//! cargo nextest run -E 'test(render_cost_curve)' --run-ignored ignored-only \
//!     --no-capture
//! ```
//!
//! Per point: a fresh isolated server (throwaway XDG, own sockets — touching a
//! live session is impossible by construction), 20 panes each producing ~20
//! lines/second, N handshaked app clients draining frames, an 8 s warmup, and
//! a 25 s measurement window. Reported per point: server CPU (utime+stime
//! delta over the window), bytes written by the server process (wchar delta),
//! frames received per client, and the machine's 1-minute load average as a
//! noise disclosure.

mod support;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use support::{
    cleanup_test_base, client_handshake, register_runtime_dir, register_spawned_herdr_pid,
    unregister_spawned_herdr_pid, wait_for_socket,
};

struct SpawnedHerdr {
    _master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl Drop for SpawnedHerdr {
    fn drop(&mut self) {
        let pid = self.child.process_id();
        let _ = self.child.kill();
        unregister_spawned_herdr_pid(pid);
    }
}

fn unique_test_dir(tag: &str) -> PathBuf {
    PathBuf::from(format!(
        "/tmp/hperf-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

fn spawn_server(config_home: &Path, runtime_dir: &Path, api_socket: &Path) -> SpawnedHerdr {
    fs::create_dir_all(config_home.join("herdr")).unwrap();
    fs::create_dir_all(runtime_dir).unwrap();
    fs::write(
        config_home.join("herdr/config.toml"),
        "onboarding = false\n",
    )
    .unwrap();

    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_herdr"));
    cmd.arg("server");
    cmd.env("XDG_CONFIG_HOME", config_home);
    cmd.env("XDG_RUNTIME_DIR", runtime_dir);
    cmd.env("HERDR_SOCKET_PATH", api_socket);
    cmd.env(
        "HERDR_CLIENT_SOCKET_PATH",
        runtime_dir.join("herdr-client.sock"),
    );
    cmd.env("SHELL", "/bin/sh");

    let child = pair.slave.spawn_command(cmd).unwrap();
    register_spawned_herdr_pid(child.process_id());
    SpawnedHerdr {
        _master: pair.master,
        child,
    }
}

fn request(socket_path: &Path, request: serde_json::Value) -> serde_json::Value {
    let mut stream = UnixStream::connect(socket_path).expect("connect api socket");
    let text = request.to_string();
    stream.write_all(text.as_bytes()).unwrap();
    stream.write_all(b"\n").unwrap();
    stream.flush().unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    serde_json::from_str(&line).expect("api response json")
}

fn wait_for_api(socket_path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(mut stream) = UnixStream::connect(socket_path) {
            let ping = serde_json::json!({"id":"perf:ping","method":"ping","params":{}});
            if stream.write_all(ping.to_string().as_bytes()).is_ok()
                && stream.write_all(b"\n").is_ok()
            {
                let mut line = String::new();
                if BufReader::new(stream).read_line(&mut line).is_ok() && line.contains("result") {
                    return;
                }
            }
        }
        assert!(Instant::now() < deadline, "api socket never became ready");
        thread::sleep(Duration::from_millis(100));
    }
}

/// 20 panes across 4 workspaces, each running a ~20 Hz line producer.
fn build_synthetic_load(api_socket: &Path) {
    for ws in 0..4 {
        let created = request(
            api_socket,
            serde_json::json!({
                "id": format!("perf:ws:{ws}"),
                "method": "workspace.create",
                "params": {"cwd": "/tmp", "focus": ws == 0}
            }),
        );
        let root = created["result"]["root_pane"]["pane_id"]
            .as_str()
            .expect("workspace root pane")
            .to_string();
        let mut pane_ids = vec![root.clone()];
        for split in 0..4 {
            let response = request(
                api_socket,
                serde_json::json!({
                    "id": format!("perf:split:{ws}:{split}"),
                    "method": "pane.split",
                    "params": {
                        "target_pane_id": pane_ids[split % pane_ids.len()],
                        "direction": if split % 2 == 0 { "right" } else { "down" },
                        "focus": false
                    }
                }),
            );
            let id = response["result"]["pane"]["pane_id"]
                .as_str()
                .expect("split pane id")
                .to_string();
            pane_ids.push(id);
        }
        for pane_id in &pane_ids {
            // The command line spells the loop plainly; echoed once, it is not
            // matched by anything — the measurement reads counters, not text.
            let spinner = "i=0; while :; do i=$((i+1)); echo p$$-$i; sleep 0.05; done";
            request(
                api_socket,
                serde_json::json!({
                    "id": format!("perf:spin:{pane_id}"),
                    "method": "pane.send_input",
                    "params": {"pane_id": pane_id, "text": spinner, "keys": ["Enter"]}
                }),
            );
        }
    }
}

struct DrainedClient {
    stop: Arc<AtomicBool>,
    bytes: Arc<AtomicU64>,
    frames: Arc<AtomicU64>,
    handle: thread::JoinHandle<()>,
}

/// Handshake as a real app client and keep draining frames off the socket.
fn attach_draining_client(client_socket: &Path, protocol: u32) -> DrainedClient {
    let mut stream = UnixStream::connect(client_socket).expect("connect client socket");
    let (server_protocol, error) =
        client_handshake(&mut stream, protocol, 120, 36).expect("client handshake");
    assert_eq!(server_protocol, protocol);
    assert!(error.is_none(), "handshake refused: {error:?}");

    let stop = Arc::new(AtomicBool::new(false));
    let bytes = Arc::new(AtomicU64::new(0));
    let frames = Arc::new(AtomicU64::new(0));
    let thread_stop = stop.clone();
    let thread_bytes = bytes.clone();
    let thread_frames = frames.clone();
    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    let handle = thread::spawn(move || {
        let mut len_buf = [0u8; 4];
        let mut payload = Vec::new();
        while !thread_stop.load(Ordering::Acquire) {
            match stream.read_exact(&mut len_buf) {
                Ok(()) => {
                    let len = u32::from_le_bytes(len_buf) as usize;
                    payload.resize(len, 0);
                    // The payload follows immediately; short timeouts mid-frame
                    // would tear the stream, so this read retries until done.
                    let mut off = 0;
                    while off < len {
                        match stream.read(&mut payload[off..]) {
                            Ok(0) => return,
                            Ok(n) => off += n,
                            Err(err)
                                if err.kind() == std::io::ErrorKind::WouldBlock
                                    || err.kind() == std::io::ErrorKind::TimedOut =>
                            {
                                if thread_stop.load(Ordering::Acquire) {
                                    return;
                                }
                            }
                            Err(_) => return,
                        }
                    }
                    thread_bytes.fetch_add(4 + len as u64, Ordering::Relaxed);
                    thread_frames.fetch_add(1, Ordering::Relaxed);
                }
                Err(err)
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        || err.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => return,
            }
        }
    });
    DrainedClient {
        stop,
        bytes,
        frames,
        handle,
    }
}

fn proc_cpu_ticks(pid: u32) -> u64 {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    // Field 2 (comm) may contain spaces; parse after the closing paren.
    let after = stat.rsplit_once(')').map(|(_, rest)| rest).unwrap_or("");
    let fields: Vec<&str> = after.split_whitespace().collect();
    // After the paren: field index 11 = utime, 12 = stime (0-based here).
    let utime: u64 = fields.get(11).and_then(|f| f.parse().ok()).unwrap_or(0);
    let stime: u64 = fields.get(12).and_then(|f| f.parse().ok()).unwrap_or(0);
    utime + stime
}

fn proc_wchar(pid: u32) -> u64 {
    fs::read_to_string(format!("/proc/{pid}/io"))
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("wchar: "))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0)
}

fn loadavg_1m() -> String {
    fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or("?")
        .to_string()
}

#[test]
#[ignore = "measurement harness, run explicitly"]
fn render_cost_curve() {
    let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let warmup = Duration::from_secs(8);
    let window = Duration::from_secs(25);

    println!();
    println!("| clients | cpu % | MB/s written | frames/s/client | load1m |");
    println!("|---|---|---|---|---|");

    for &n_clients in &[0usize, 1, 2, 4, 8, 10] {
        let base = unique_test_dir(&format!("n{n_clients}"));
        let config_home = base.join("config");
        let runtime_dir = base.join("runtime");
        let api_socket = runtime_dir.join("herdr.sock");
        let client_socket = runtime_dir.join("herdr-client.sock");

        let spawned = spawn_server(&config_home, &runtime_dir, &api_socket);
        let server_pid = spawned.child.process_id().expect("server pid");
        wait_for_socket(&api_socket, Duration::from_secs(10));
        register_runtime_dir(&runtime_dir);
        wait_for_api(&api_socket, Duration::from_secs(10));

        build_synthetic_load(&api_socket);

        let protocol = request(
            &api_socket,
            serde_json::json!({"id":"perf:proto","method":"ping","params":{}}),
        )["result"]["protocol"]
            .as_u64()
            .expect("protocol") as u32;

        let clients: Vec<DrainedClient> = (0..n_clients)
            .map(|_| attach_draining_client(&client_socket, protocol))
            .collect();

        thread::sleep(warmup);

        let cpu_before = proc_cpu_ticks(server_pid);
        let wchar_before = proc_wchar(server_pid);
        let frames_before: u64 = clients
            .iter()
            .map(|c| c.frames.load(Ordering::Relaxed))
            .sum();
        let started = Instant::now();
        thread::sleep(window);
        let elapsed = started.elapsed().as_secs_f64();
        let cpu_after = proc_cpu_ticks(server_pid);
        let wchar_after = proc_wchar(server_pid);
        let frames_after: u64 = clients
            .iter()
            .map(|c| c.frames.load(Ordering::Relaxed))
            .sum();

        let cpu_pct = (cpu_after - cpu_before) as f64 / clk_tck / elapsed * 100.0;
        let mbps = (wchar_after - wchar_before) as f64 / elapsed / 1_048_576.0;
        let frames_per_client = if n_clients == 0 {
            0.0
        } else {
            (frames_after - frames_before) as f64 / elapsed / n_clients as f64
        };

        println!(
            "| {n_clients} | {cpu_pct:.1} | {mbps:.2} | {frames_per_client:.1} | {} |",
            loadavg_1m()
        );

        for client in &clients {
            client.stop.store(true, Ordering::Release);
        }
        for client in clients {
            let _ = client.handle.join();
            let _ = client.bytes; // counted via frames table; kept for debugging
        }
        let _ = request(
            &api_socket,
            serde_json::json!({"id":"perf:stop","method":"server.stop","params":{}}),
        );
        drop(spawned);
        cleanup_test_base(&base);
    }
}
