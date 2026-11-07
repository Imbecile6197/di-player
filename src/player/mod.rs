use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Buffering,
    Playing,
    Paused,
    Error,
}

pub struct MpvPlayer {
    inner: Arc<Mutex<PlayerInner>>,
}

struct PlayerInner {
    child: Option<Child>,
    socket: Option<PathBuf>,
    state: PlaybackState,
    volume: f64,
    current_url: Option<String>,
}

impl MpvPlayer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlayerInner {
                child: None,
                socket: None,
                state: PlaybackState::Stopped,
                volume: 0.85,
                current_url: None,
            })),
        }
    }

    pub fn state(&self) -> PlaybackState {
        self.inner.lock().state
    }

    pub fn volume(&self) -> f64 {
        self.inner.lock().volume
    }

    pub fn ensure_running(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        if let Some(child) = inner.child.as_mut() {
            match child.try_wait() {
                Ok(None) => return Ok(()),
                Ok(Some(_)) | Err(_) => {
                    inner.child = None;
                    if let Some(sock) = inner.socket.take() {
                        let _ = std::fs::remove_file(sock);
                    }
                }
            }
        }

        let runtime = dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let socket = runtime.join(format!("di-player-mpv-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);

        let child = Command::new("mpv")
            .args([
                "--no-video",
                "--no-terminal",
                "--idle=yes",
                "--gapless-audio=yes",
                "--cache=yes",
                "--demuxer-max-bytes=8MiB",
                "--audio-client-name=DI Player",
                "--title=DI Player",
            ])
            .arg(format!("--input-ipc-server={}", socket.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to start mpv — is mpv installed?")?;

        // Wait until the IPC socket accepts connections.
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if UnixStream::connect(&socket).is_ok() {
                break;
            }
            if Instant::now() > deadline {
                let _ = child;
                bail!("mpv IPC socket did not become ready");
            }
            thread::sleep(Duration::from_millis(40));
        }

        inner.child = Some(child);
        inner.socket = Some(socket);
        Ok(())
    }

    fn ipc_raw(socket: &PathBuf, command: serde_json::Value) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(socket).context("connect to mpv IPC")?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;

        let mut line = command.to_string();
        line.push('\n');
        stream.write_all(line.as_bytes())?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        loop {
            response.clear();
            reader.read_line(&mut response)?;
            if response.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(response.trim())?;
            // Event lines have "event"; replies have "error" or "data".
            if value.get("event").is_some() && value.get("error").is_none() && value.get("request_id").is_none() {
                continue;
            }
            return Ok(value);
        }
    }

    fn with_ipc<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&PathBuf) -> Result<T>,
    {
        self.ensure_running()?;
        let socket = {
            let inner = self.inner.lock();
            inner
                .socket
                .clone()
                .context("mpv socket missing")?
        };
        f(&socket)
    }

    pub fn play_url(&self, url: &str) -> Result<()> {
        self.with_ipc(|socket| {
            let cmd = serde_json::json!({
                "command": ["loadfile", url, "replace"]
            });
            let resp = Self::ipc_raw(socket, cmd)?;
            if resp.get("error").and_then(|e| e.as_str()) != Some("success") {
                bail!("mpv loadfile failed: {resp}");
            }
            Ok(())
        })?;

        let mut inner = self.inner.lock();
        inner.current_url = Some(url.to_string());
        inner.state = PlaybackState::Playing;
        let vol = (inner.volume * 100.0).round() as i64;
        drop(inner);
        let _ = self.set_volume_percent(vol);
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        self.with_ipc(|socket| {
            let cmd = serde_json::json!({ "command": ["set_property", "pause", true] });
            let _ = Self::ipc_raw(socket, cmd)?;
            Ok(())
        })?;
        self.inner.lock().state = PlaybackState::Paused;
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        self.with_ipc(|socket| {
            let cmd = serde_json::json!({ "command": ["set_property", "pause", false] });
            let _ = Self::ipc_raw(socket, cmd)?;
            Ok(())
        })?;
        self.inner.lock().state = PlaybackState::Playing;
        Ok(())
    }

    pub fn toggle_pause(&self) -> Result<PlaybackState> {
        let state = self.state();
        match state {
            PlaybackState::Playing => {
                self.pause()?;
                Ok(PlaybackState::Paused)
            }
            PlaybackState::Paused => {
                self.resume()?;
                Ok(PlaybackState::Playing)
            }
            other => Ok(other),
        }
    }

    pub fn stop(&self) -> Result<()> {
        if self.inner.lock().child.is_none() {
            self.inner.lock().state = PlaybackState::Stopped;
            return Ok(());
        }
        let _ = self.with_ipc(|socket| {
            let cmd = serde_json::json!({ "command": ["stop"] });
            let _ = Self::ipc_raw(socket, cmd)?;
            Ok(())
        });
        let mut inner = self.inner.lock();
        inner.state = PlaybackState::Stopped;
        inner.current_url = None;
        Ok(())
    }

    pub fn set_volume(&self, volume: f64) -> Result<()> {
        let volume = volume.clamp(0.0, 1.0);
        self.inner.lock().volume = volume;
        self.set_volume_percent((volume * 100.0).round() as i64)
    }

    fn set_volume_percent(&self, percent: i64) -> Result<()> {
        if self.inner.lock().child.is_none() {
            return Ok(());
        }
        self.with_ipc(|socket| {
            let cmd = serde_json::json!({ "command": ["set_property", "volume", percent] });
            let _ = Self::ipc_raw(socket, cmd)?;
            Ok(())
        })
    }

    pub fn shutdown(&self) {
        let mut inner = self.inner.lock();
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(sock) = inner.socket.take() {
            let _ = std::fs::remove_file(sock);
        }
        inner.state = PlaybackState::Stopped;
        inner.current_url = None;
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
