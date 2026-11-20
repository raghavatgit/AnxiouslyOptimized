pub mod cleaner;
pub mod debloat;
pub mod emulator;
pub mod gamemode;
pub mod installer;
pub mod maintenance;
pub mod network;
pub mod runtimes;
pub mod safety;
pub mod tweaks;
pub mod uninstaller;
pub mod updater;
pub mod valorant;

use crate::models::{AppSettings, LogEntry, LogLevel};
use chrono::Local;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn app_data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"));
    base.join("OptimizerByRaghav")
}

pub fn journals_dir() -> PathBuf {
    app_data_dir().join("journals")
}

pub fn file_backups_dir() -> PathBuf {
    app_data_dir().join("file_backups")
}

pub fn ensure_app_dirs() {
    let _ = std::fs::create_dir_all(app_data_dir());
    let _ = std::fs::create_dir_all(journals_dir());
    let _ = std::fs::create_dir_all(file_backups_dir());
}

pub fn settings_path() -> PathBuf {
    app_data_dir().join("settings.json")
}

pub fn gamemode_config_path() -> PathBuf {
    app_data_dir().join("gamemode.json")
}

pub fn system_drive_root() -> String {
    let drv = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let trimmed = drv.trim_end_matches('\\');
    format!(r"{}\", trimmed)
}

pub fn now_iso() -> String {
    Local::now().to_rfc3339()
}

pub fn now_hms() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

pub type SharedLogs = Arc<Mutex<Vec<LogEntry>>>;

pub fn push_log(logs: &SharedLogs, level: LogLevel, msg: impl Into<String>) {
    let message = msg.into();
    if let Ok(mut guard) = logs.lock() {
        guard.push(LogEntry {
            timestamp: now_hms(),
            level,
            message,
        });
        if guard.len() > 2000 {
            let drain = guard.len() - 2000;
            guard.drain(0..drain);
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else if b < TB {
        format!("{:.2} GB", b / GB)
    } else {
        format!("{:.2} TB", b / TB)
    }
}

pub fn to_wide_null(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

pub fn from_wide_null(slice: &[u16]) -> String {
    let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..end]).trim().to_string()
}

pub fn decode_output_bytes(bytes: &[u8], utf16le: bool) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if utf16le || (bytes.len() >= 4 && bytes[1] == 0 && bytes[3] == 0) {
        let mut u16s = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            u16s.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        String::from_utf16_lossy(&u16s)
            .trim_start_matches('\u{FEFF}')
            .to_string()
    } else {
        String::from_utf8_lossy(bytes)
            .trim_start_matches('\u{FEFF}')
            .to_string()
    }
}

/// Runs a command hidden with concurrent stdout+stderr drain threads and strict timeout + kill-on-timeout.
pub fn run_command_timeout(
    exe: &str,
    args: &[&str],
    timeout_ms: u64,
) -> Result<CommandOutput, String> {
    run_command_timeout_decoded(exe, args, timeout_ms, false)
}

pub fn run_command_timeout_decoded(
    exe: &str,
    args: &[&str],
    timeout_ms: u64,
    utf16le: bool,
) -> Result<CommandOutput, String> {
    let mut child = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {exe}: {e}"))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let out_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = stdout_pipe {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    let err_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = stderr_pipe {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let exit_code;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                exit_code = status.code().unwrap_or(-1);
                break;
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out_handle.join();
                    let _ = err_handle.join();
                    return Err(format!("Command '{exe}' timed out after {timeout_ms} ms"));
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = out_handle.join();
                let _ = err_handle.join();
                return Err(format!("Failed waiting on '{exe}': {e}"));
            }
        }
    }

    let out_bytes = out_handle.join().unwrap_or_default();
    let err_bytes = err_handle.join().unwrap_or_default();

    Ok(CommandOutput {
        exit_code,
        stdout: decode_output_bytes(&out_bytes, utf16le),
        stderr: decode_output_bytes(&err_bytes, utf16le),
    })
}

/// Streams command output line-by-line (supporting UTF-16LE tools such as sfc.exe) with cancellation.
pub fn run_command_streaming<F>(
    exe: &str,
    args: &[&str],
    utf16le: bool,
    on_line: F,
    cancel: &AtomicBool,
    timeout_ms: u64,
) -> Result<i32, String>
where
    F: Fn(String) + Send + Sync + 'static,
{
    let mut child = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {exe}: {e}"))?;

    let cb = Arc::new(on_line);
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let cb_out = Arc::clone(&cb);
    let out_handle = std::thread::spawn(move || {
        if let Some(p) = stdout_pipe {
            stream_pipe_lines(p, utf16le, &*cb_out);
        }
    });

    let cb_err = Arc::clone(&cb);
    let err_handle = std::thread::spawn(move || {
        if let Some(p) = stderr_pipe {
            stream_pipe_lines(p, utf16le, &*cb_err);
        }
    });

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let code = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_handle.join();
            let _ = err_handle.join();
            return Err(format!("Command '{exe}' cancelled by user"));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out_handle.join();
                    let _ = err_handle.join();
                    return Err(format!("Command '{exe}' timed out after {timeout_ms} ms"));
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = out_handle.join();
                let _ = err_handle.join();
                return Err(format!("Error waiting on '{exe}': {e}"));
            }
        }
    };

    let _ = out_handle.join();
    let _ = err_handle.join();
    Ok(code)
}

fn stream_pipe_lines<R: Read>(mut reader: R, utf16le: bool, on_line: &dyn Fn(String)) {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&chunk[..n]);
                if utf16le {
                    while let Some(pos) = find_utf16_newline(&raw) {
                        let line_bytes: Vec<u8> = raw.drain(..pos).collect();
                        // drain newline bytes (2 for \n, or 4 if \r\n)
                        if raw.len() >= 2 && raw[0] == b'\n' && raw[1] == 0 {
                            raw.drain(..2);
                        }
                        let text = decode_output_bytes(&line_bytes, true).trim().to_string();
                        if !text.is_empty() {
                            on_line(text);
                        }
                    }
                } else {
                    while let Some(pos) = raw.iter().position(|&b| b == b'\n' || b == b'\r') {
                        let line_bytes: Vec<u8> = raw.drain(..pos).collect();
                        while !raw.is_empty() && (raw[0] == b'\r' || raw[0] == b'\n') {
                            raw.remove(0);
                        }
                        let text = String::from_utf8_lossy(&line_bytes).trim().to_string();
                        if !text.is_empty() {
                            on_line(text);
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
    if !raw.is_empty() {
        let rem = decode_output_bytes(&raw, utf16le).trim().to_string();
        if !rem.is_empty() {
            on_line(rem);
        }
    }
}

fn find_utf16_newline(bytes: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 1 < bytes.len() {
        let code = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if code == b'\n' as u16 {
            return Some(i);
        }
        i += 2;
    }
    None
}

pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn encode_powershell_script(script: &str) -> String {
    let mut utf16 = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    base64_encode(&utf16)
}

pub fn run_powershell_encoded(script: &str, timeout_ms: u64) -> Result<CommandOutput, String> {
    let encoded = encode_powershell_script(script);
    run_command_timeout(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encoded,
        ],
        timeout_ms,
    )
}

pub fn is_process_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elev as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
}

pub fn load_settings() -> AppSettings {
    ensure_app_dirs();
    let path = settings_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    ensure_app_dirs();
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed serializing settings: {e}"))?;
    std::fs::write(settings_path(), json).map_err(|e| format!("Failed writing settings: {e}"))
}
