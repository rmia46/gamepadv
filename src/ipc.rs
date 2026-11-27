use std::fs::{self, Permissions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::{chown, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::engine::{GamepadState, MouseStickConfig};
use crate::profile::{Profile, ProfileConfig};

/// Commands sent from the unprivileged GUI to the `--daemon` or tray host process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcCommand {
    SetActive(bool),
    UpdateProfile(ProfileConfig),
    UnlockPkexec,
    Shutdown,
}

/// Snapshot streamed from the `--daemon` / tray host process to the GUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcSnapshot {
    pub active: bool,
    #[serde(default)]
    pub unlocking: bool,
    pub latest_state: GamepadState,
    pub keyboard_count: usize,
    pub mouse_count: usize,
    pub status_message: String,
    pub permission_error: Option<String>,
    #[serde(default)]
    pub last_key: Option<String>,
    #[serde(default)]
    pub mouse_delta: (i32, i32),
}

/// Returns the current process UID and GID via `/proc/self` metadata (no `libc` needed).
pub fn current_uid_gid() -> io::Result<(u32, u32)> {
    let meta = fs::metadata("/proc/self")?;
    Ok((meta.uid(), meta.gid()))
}

/// Picks a per-user/per-PID Unix socket path for `pkexec` daemon IPC.
pub fn default_socket_path() -> PathBuf {
    let pid = std::process::id();
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let dir = PathBuf::from(runtime_dir);
        if dir.is_dir() {
            return dir.join(format!("gamepadv-{pid}.sock"));
        }
    }
    let uid = current_uid_gid().map(|(u, _)| u).unwrap_or(0);
    PathBuf::from(format!("/tmp/gamepadv-{uid}-{pid}.sock"))
}

/// RAII guard that removes the Unix socket file when dropped.
pub struct SocketFileGuard {
    path: PathBuf,
}

impl SocketFileGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for SocketFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Binds a Unix socket at `path`, restricts permissions to `0600`, and optionally
/// `chown`s it to `(owner_uid, owner_gid)` so only the launching user can connect.
pub fn bind_restricted_socket(
    path: &Path,
    owner: Option<(u32, u32)>,
) -> io::Result<(UnixListener, SocketFileGuard)> {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, Permissions::from_mode(0o600))?;
    if let Some((uid, gid)) = owner {
        chown(path, Some(uid), Some(gid))?;
    }
    Ok((listener, SocketFileGuard::new(path.to_path_buf())))
}

/// Writes one newline-delimited JSON message.
pub fn write_json_line<T: Serialize>(stream: &mut UnixStream, msg: &T) -> io::Result<()> {
    let mut line = serde_json::to_string(msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    line.push('\n');
    stream.write_all(line.as_bytes())?;
    stream.flush()
}

/// Reads one newline-delimited JSON message from a buffered Unix stream.
pub fn read_json_line<T: for<'de> Deserialize<'de>>(
    reader: &mut BufReader<UnixStream>,
) -> io::Result<Option<T>> {
    let mut line = String::new();
    let n = reader.read_line(&mut line)?;
    if n == 0 {
        return Ok(None);
    }
    let parsed = serde_json::from_str::<T>(line.trim_end())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(parsed))
}

/// Polls until `socket_path` is connectable or `timeout` elapses (while `pkexec` prompts).
pub fn wait_for_socket(socket_path: &Path, timeout: Duration) -> io::Result<UnixStream> {
    let deadline = Instant::now() + timeout;
    loop {
        match UnixStream::connect(socket_path) {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                if Instant::now() >= deadline {
                    return Err(err);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Helper to convert `(Profile, MouseStickConfig)` into `ProfileConfig`.
pub fn to_profile_config(profile: &Profile, mouse: &MouseStickConfig) -> ProfileConfig {
    ProfileConfig::from_parts(profile, mouse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::default_3d_action;

    #[test]
    fn ipc_command_and_snapshot_round_trip() {
        let dir = std::env::temp_dir().join(format!("gamepadv-ipc-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("test.sock");

        let (listener, _guard) = bind_restricted_socket(&sock_path, None).unwrap();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let cmd: IpcCommand = read_json_line(&mut reader).unwrap().unwrap();
            assert!(matches!(cmd, IpcCommand::SetActive(true)));

            let snap = IpcSnapshot {
                active: true,
                unlocking: false,
                latest_state: GamepadState {
                    south: true,
                    right_x: 12345,
                    ..GamepadState::default()
                },
                keyboard_count: 1,
                mouse_count: 1,
                status_message: "ACTIVE".to_string(),
                permission_error: None,
                last_key: Some("Space (down)".to_string()),
                mouse_delta: (5, -2),
            };
            write_json_line(&mut stream, &snap).unwrap();
        });

        let mut client = wait_for_socket(&sock_path, Duration::from_secs(2)).unwrap();
        write_json_line(&mut client, &IpcCommand::SetActive(true)).unwrap();
        let mut reader = BufReader::new(client);
        let snap: IpcSnapshot = read_json_line(&mut reader).unwrap().unwrap();
        assert!(snap.active);
        assert!(snap.latest_state.south);
        assert_eq!(snap.latest_state.right_x, 12345);

        // Also verify ProfileConfig serialization inside IpcCommand.
        let cfg = to_profile_config(&default_3d_action(), &MouseStickConfig::default());
        let json = serde_json::to_string(&IpcCommand::UpdateProfile(cfg)).unwrap();
        let decoded: IpcCommand = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, IpcCommand::UpdateProfile(_)));

        server.join().unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
