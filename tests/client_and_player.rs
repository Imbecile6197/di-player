use di_player_test_support::*;

// The binary crate isn't a lib, so we duplicate a thin harness by invoking
// library-free checks against the same endpoints the app uses, plus mpv IPC.

mod di_player_test_support {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    pub fn http_get(url: &str) -> (u16, String) {
        let output = Command::new("curl")
            .args(["-sS", "-w", "\n%{http_code}", url])
            .output()
            .expect("curl");
        let text = String::from_utf8_lossy(&output.stdout);
        let code = text
            .lines()
            .last()
            .unwrap()
            .parse::<u16>()
            .unwrap_or(0);
        let body = text
            .rsplit_once('\n')
            .map(|(b, _)| b.to_string())
            .unwrap_or_default();
        (code, body)
    }

    pub fn start_mpv() -> (std::process::Child, PathBuf) {
        let sock = std::env::temp_dir().join(format!("di-player-test-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock);
        let child = Command::new("mpv")
            .args([
                "--no-video",
                "--no-terminal",
                "--idle=yes",
                &format!("--input-ipc-server={}", sock.display()),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("mpv");
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if UnixStream::connect(&sock).is_ok() {
                return (child, sock);
            }
            thread::sleep(Duration::from_millis(40));
        }
        panic!("mpv socket not ready");
    }

    pub fn mpv_cmd(sock: &PathBuf, cmd: serde_json::Value) -> serde_json::Value {
        let mut stream = UnixStream::connect(sock).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut line = cmd.to_string();
        line.push('\n');
        stream.write_all(line.as_bytes()).unwrap();
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        loop {
            response.clear();
            reader.read_line(&mut response).unwrap();
            let value: serde_json::Value = serde_json::from_str(response.trim()).unwrap();
            if value.get("event").is_some() && value.get("error").is_none() {
                continue;
            }
            return value;
        }
    }
}

#[test]
fn channel_filters_contain_popular_and_all() {
    let (code, body) = http_get(
        "https://api.audioaddict.com/v1/di/channels",
    );
    // wrong URL on purpose? Fix:
    let _ = (code, body);
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "-H",
            "Authorization: Basic ZXBoZW1lcm9uOmRheWVpcGgwbmVAcHA=",
            "https://api.audioaddict.com/v1/di/channel_filters",
        ])
        .output()
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let names: Vec<&str> = value
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.iter().any(|n| *n == "Popular"));
    assert!(names.iter().any(|n| *n == "All"));
}

#[test]
fn pls_playlist_contains_file1() {
    let output = std::process::Command::new("curl")
        .args(["-sS", "https://listen.di.fm/premium_high/trance.pls"])
        .output()
        .unwrap();
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(body.contains("File1="), "PLS body: {body}");
    assert!(body.contains("prem"), "expected premium host in {body}");
}

#[test]
fn mpv_ipc_idle_and_stop() {
    let (mut child, sock) = start_mpv();
    let resp = mpv_cmd(
        &sock,
        serde_json::json!({"command": ["get_property", "idle-active"]}),
    );
    assert_eq!(resp["error"], "success");
    assert_eq!(resp["data"], true);

    let stop = mpv_cmd(&sock, serde_json::json!({"command": ["quit"]}));
    assert_eq!(stop["error"], "success");
    let _ = child.wait();
    let _ = std::fs::remove_file(sock);
}
