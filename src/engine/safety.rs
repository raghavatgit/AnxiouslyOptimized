use crate::engine::{
    ensure_app_dirs, file_backups_dir, format_bytes, journals_dir, now_iso,
    run_command_timeout, run_powershell_encoded, system_drive_root, to_wide_null,
};
use crate::models::{
    RegHive, RestorePointInfo, TransactionJournal, TransactionStep, TypedRegValue, VssHealth,
};
use crate::system::platform::{format_dmtf_datetime, parse_guid};
use crate::system::{var_str, var_u32};
use chrono::Local;
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};
use windows::Win32::System::Power::{PowerSetActiveScheme, PowerWriteACValueIndex};
use windows::Win32::System::Services::{
    ChangeServiceConfigW, CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW,
    QueryServiceConfigW, QueryServiceStatusEx, ENUM_SERVICE_TYPE, QUERY_SERVICE_CONFIGW,
    SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_CHANGE_CONFIG, SERVICE_CONTROL_STOP,
    SERVICE_ERROR, SERVICE_NO_CHANGE, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS,
    SERVICE_START_TYPE, SERVICE_STATUS, SERVICE_STATUS_PROCESS, SERVICE_STOP,
};
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, KEY_WRITE, REG_BINARY,
    REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_QWORD, REG_SZ,
};
use winreg::{RegKey, RegValue};
use wmi::{COMLibrary, Variant, WMIConnection};

const SYSTEM_RESTORE_SUBKEY: &str =
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
const SR_FREQ_VALUE: &str = "SystemRestorePointCreationFrequency";

pub fn open_hive_root(hive: RegHive) -> RegKey {
    match hive {
        RegHive::Hklm => RegKey::predef(HKEY_LOCAL_MACHINE),
        RegHive::Hkcu => RegKey::predef(HKEY_CURRENT_USER),
    }
}

pub fn reg_subkey_exists(hive: RegHive, subkey: &str) -> bool {
    open_hive_root(hive)
        .open_subkey_with_flags(subkey, KEY_READ | KEY_WOW64_64KEY)
        .is_ok()
}

pub fn read_typed_reg_value(
    hive: RegHive,
    subkey: &str,
    value_name: &str,
) -> Option<TypedRegValue> {
    let key = open_hive_root(hive)
        .open_subkey_with_flags(subkey, KEY_READ | KEY_WOW64_64KEY)
        .ok()?;
    let raw = key.get_raw_value(value_name).ok()?;
    raw_to_typed(&raw)
}

pub fn read_reg_dword(hive: RegHive, subkey: &str, value_name: &str) -> Option<u32> {
    match read_typed_reg_value(hive, subkey, value_name) {
        Some(TypedRegValue::Dword(v)) => Some(v),
        _ => None,
    }
}

pub fn read_reg_string(hive: RegHive, subkey: &str, value_name: &str) -> Option<String> {
    match read_typed_reg_value(hive, subkey, value_name) {
        Some(TypedRegValue::String(s)) | Some(TypedRegValue::ExpandString(s)) => Some(s),
        _ => None,
    }
}

pub fn raw_to_typed(raw: &RegValue) -> Option<TypedRegValue> {
    match raw.vtype {
        REG_DWORD => {
            if raw.bytes.len() >= 4 {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&raw.bytes[..4]);
                Some(TypedRegValue::Dword(u32::from_le_bytes(arr)))
            } else {
                None
            }
        }
        REG_QWORD => {
            if raw.bytes.len() >= 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&raw.bytes[..8]);
                Some(TypedRegValue::Qword(u64::from_le_bytes(arr)))
            } else {
                None
            }
        }
        REG_SZ => Some(TypedRegValue::String(bytes_to_utf16_string(&raw.bytes))),
        REG_EXPAND_SZ => Some(TypedRegValue::ExpandString(bytes_to_utf16_string(
            &raw.bytes,
        ))),
        REG_MULTI_SZ => {
            let u16s: Vec<u16> = raw
                .bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let mut items = Vec::new();
            for part in u16s.split(|&w| w == 0) {
                if !part.is_empty() {
                    items.push(String::from_utf16_lossy(part));
                }
            }
            Some(TypedRegValue::MultiString(items))
        }
        REG_BINARY => Some(TypedRegValue::Binary(raw.bytes.clone())),
        _ => Some(TypedRegValue::Binary(raw.bytes.clone())),
    }
}

pub fn typed_to_raw(val: &TypedRegValue) -> RegValue {
    match val {
        TypedRegValue::Dword(v) => RegValue {
            bytes: v.to_le_bytes().to_vec(),
            vtype: REG_DWORD,
        },
        TypedRegValue::Qword(v) => RegValue {
            bytes: v.to_le_bytes().to_vec(),
            vtype: REG_QWORD,
        },
        TypedRegValue::String(s) => RegValue {
            bytes: utf16_null_bytes(s),
            vtype: REG_SZ,
        },
        TypedRegValue::ExpandString(s) => RegValue {
            bytes: utf16_null_bytes(s),
            vtype: REG_EXPAND_SZ,
        },
        TypedRegValue::MultiString(list) => {
            let mut bytes = Vec::new();
            for item in list {
                for u in item.encode_utf16() {
                    bytes.extend_from_slice(&u.to_le_bytes());
                }
                bytes.extend_from_slice(&[0, 0]);
            }
            bytes.extend_from_slice(&[0, 0]);
            RegValue {
                bytes,
                vtype: REG_MULTI_SZ,
            }
        }
        TypedRegValue::Binary(b) => RegValue {
            bytes: b.clone(),
            vtype: REG_BINARY,
        },
    }
}

fn bytes_to_utf16_string(bytes: &[u8]) -> String {
    let u16s: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let end = u16s.iter().position(|&c| c == 0).unwrap_or(u16s.len());
    String::from_utf16_lossy(&u16s[..end])
}

fn utf16_null_bytes(s: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((s.len() + 1) * 2);
    for u in s.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

pub fn write_typed_reg_value(
    hive: RegHive,
    subkey: &str,
    value_name: &str,
    val: &TypedRegValue,
) -> Result<(), String> {
    let root = open_hive_root(hive);
    let (key, _) = root
        .create_subkey_with_flags(subkey, KEY_WRITE | KEY_WOW64_64KEY)
        .map_err(|e| format!(r"Registry create/open {}\{} failed: {e}", hive.cli_prefix(), subkey))?;
    let raw = typed_to_raw(val);
    key.set_raw_value(value_name, &raw).map_err(|e| {
        format!(
            r"Registry write {}\{}\{} failed: {e}",
            hive.cli_prefix(),
            subkey,
            value_name
        )
    })
}

pub fn delete_reg_value(hive: RegHive, subkey: &str, value_name: &str) -> Result<(), String> {
    let root = open_hive_root(hive);
    let key = match root.open_subkey_with_flags(subkey, KEY_WRITE | KEY_WOW64_64KEY) {
        Ok(k) => k,
        Err(_) => return Ok(()),
    };
    match key.delete_value(value_name) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            r"Registry delete {}\{}\{} failed: {e}",
            hive.cli_prefix(),
            subkey,
            value_name
        )),
    }
}

pub fn delete_reg_tree(hive: RegHive, subkey: &str) -> Result<(), String> {
    let root = open_hive_root(hive);
    // Split parent/leaf so HKLM 64-bit view deletes accurately.
    if let Some((parent, leaf)) = subkey.rsplit_once('\\') {
        let Ok(p_key) = root.open_subkey_with_flags(parent, KEY_WRITE | KEY_WOW64_64KEY) else {
            return Ok(());
        };
        match p_key.delete_subkey_all(leaf) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!(
                r"Registry delete tree {}\{} failed: {e}",
                hive.cli_prefix(),
                subkey
            )),
        }
    } else {
        match root.delete_subkey_all(subkey) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!(
                r"Registry delete tree {}\{} failed: {e}",
                hive.cli_prefix(),
                subkey
            )),
        }
    }
}

/// Captures exact previous value, writes ONLY first, and journals ONLY on success.
pub fn reg_set_journaled(
    journal: &mut TransactionJournal,
    hive: RegHive,
    subkey: &str,
    value_name: &str,
    val: TypedRegValue,
) -> Result<(), String> {
    let previous = read_typed_reg_value(hive, subkey, value_name);
    write_typed_reg_value(hive, subkey, value_name, &val)?;
    journal.steps.push(TransactionStep::RegistryValue {
        hive,
        subkey: subkey.to_string(),
        value_name: value_name.to_string(),
        previous,
        applied: Some(val),
    });
    Ok(())
}

/// Deletes a registry value while journaling its prior state so rollback can restore it cleanly.
pub fn reg_delete_value_journaled(
    journal: &mut TransactionJournal,
    hive: RegHive,
    subkey: &str,
    value_name: &str,
) -> Result<(), String> {
    let previous = read_typed_reg_value(hive, subkey, value_name);
    if previous.is_none() {
        return Ok(());
    }
    delete_reg_value(hive, subkey, value_name)?;
    journal.steps.push(TransactionStep::RegistryValue {
        hive,
        subkey: subkey.to_string(),
        value_name: value_name.to_string(),
        previous,
        applied: None,
    });
    Ok(())
}

/// Creates a new key (tracking `top_key_to_remove` if it didn't exist before) and writes a value.
pub fn reg_create_key_and_set_journaled(
    journal: &mut TransactionJournal,
    hive: RegHive,
    top_key_to_remove: &str,
    target_subkey: &str,
    value_name: &str,
    val: TypedRegValue,
) -> Result<(), String> {
    let top_existed = reg_subkey_exists(hive, top_key_to_remove);
    let prev = read_typed_reg_value(hive, target_subkey, value_name);
    write_typed_reg_value(hive, target_subkey, value_name, &val)?;
    if !top_existed {
        journal.steps.push(TransactionStep::RegistryKeyCreated {
            hive,
            subkey: top_key_to_remove.to_string(),
        });
    } else {
        journal.steps.push(TransactionStep::RegistryValue {
            hive,
            subkey: target_subkey.to_string(),
            value_name: value_name.to_string(),
            previous: prev,
            applied: Some(val),
        });
    }
    Ok(())
}

pub fn service_query_state(service_name: &str) -> Result<(u32, String), String> {
    unsafe {
        let scm = OpenSCManagerW(None, None, SC_MANAGER_CONNECT)
            .map_err(|e| format!("OpenSCManagerW failed: {e}"))?;
        let wide_name = to_wide_null(service_name);
        let svc_res = OpenServiceW(
            scm,
            PCWSTR(wide_name.as_ptr()),
            SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS,
        );
        let svc = match svc_res {
            Ok(s) => s,
            Err(e) => {
                let _ = CloseServiceHandle(scm);
                return Err(format!("OpenServiceW({service_name}) failed: {e}"));
            }
        };

        let mut needed = 0u32;
        let _ = QueryServiceConfigW(svc, None, 0, &mut needed);
        let mut buf = vec![0u8; needed.max(512) as usize];
        let start_type = if QueryServiceConfigW(
            svc,
            Some(buf.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW),
            buf.len() as u32,
            &mut needed,
        )
        .is_ok()
        {
            let cfg = &*(buf.as_ptr() as *const QUERY_SERVICE_CONFIGW);
            cfg.dwStartType.0
        } else {
            u32::MAX
        };

        let mut ssp = SERVICE_STATUS_PROCESS::default();
        let mut st_needed = 0u32;
        let status_text = if QueryServiceStatusEx(
            svc,
            SC_STATUS_PROCESS_INFO,
            Some(std::slice::from_raw_parts_mut(
                &mut ssp as *mut _ as *mut u8,
                std::mem::size_of::<SERVICE_STATUS_PROCESS>(),
            )),
            &mut st_needed,
        )
        .is_ok()
        {
            match ssp.dwCurrentState.0 {
                1 => "Stopped",
                2 => "Start Pending",
                3 => "Stop Pending",
                4 => "Running",
                _ => "Unknown",
            }
            .to_string()
        } else {
            "Unknown".to_string()
        };

        let _ = CloseServiceHandle(svc);
        let _ = CloseServiceHandle(scm);

        if start_type == u32::MAX {
            Err(format!("QueryServiceConfigW({service_name}) failed"))
        } else {
            Ok((start_type, status_text))
        }
    }
}

pub fn service_set_start_journaled(
    journal: &mut TransactionJournal,
    service_name: &str,
    new_start_type: u32,
    stop_now: bool,
) -> Result<(), String> {
    let (prev_start, _) = service_query_state(service_name)?;
    set_service_start_raw(service_name, new_start_type, stop_now)?;
    journal.steps.push(TransactionStep::ServiceStartMode {
        service_name: service_name.to_string(),
        previous_start_type: prev_start,
        applied_start_type: new_start_type,
    });
    Ok(())
}

pub fn set_service_start_raw(
    service_name: &str,
    new_start_type: u32,
    stop_now: bool,
) -> Result<(), String> {
    unsafe {
        let scm = OpenSCManagerW(None, None, SC_MANAGER_CONNECT)
            .map_err(|e| format!("OpenSCManagerW failed: {e}"))?;
        let wide_name = to_wide_null(service_name);
        let access = SERVICE_CHANGE_CONFIG | SERVICE_QUERY_CONFIG | if stop_now { SERVICE_STOP } else { 0 };
        let svc = match OpenServiceW(scm, PCWSTR(wide_name.as_ptr()), access) {
            Ok(s) => s,
            Err(e) => {
                let _ = CloseServiceHandle(scm);
                return Err(format!("OpenServiceW({service_name}) failed: {e}"));
            }
        };

        let res = ChangeServiceConfigW(
            svc,
            ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
            SERVICE_START_TYPE(new_start_type),
            SERVICE_ERROR(SERVICE_NO_CHANGE),
            PCWSTR::null(),
            PCWSTR::null(),
            None,
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
            PCWSTR::null(),
        )
        .map_err(|e| format!("ChangeServiceConfigW({service_name}) failed: {e}"));

        if res.is_ok() && stop_now {
            let mut st = SERVICE_STATUS::default();
            let _ = ControlService(svc, SERVICE_CONTROL_STOP, &mut st);
        }

        let _ = CloseServiceHandle(svc);
        let _ = CloseServiceHandle(scm);
        res
    }
}

pub fn file_backup_journaled(
    journal: &mut TransactionJournal,
    original_path: &Path,
) -> Result<Option<PathBuf>, String> {
    ensure_app_dirs();
    let existed = original_path.exists();
    if !existed {
        journal.steps.push(TransactionStep::FileBackup {
            original_path: original_path.to_string_lossy().to_string(),
            backup_path: String::new(),
            existed: false,
        });
        return Ok(None);
    }

    let file_name = original_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.bak".to_string());
    let stamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
    let backup_path = file_backups_dir().join(format!("{stamp}_{}_{file_name}.bak", journal.journal_id));
    std::fs::copy(original_path, &backup_path).map_err(|e| {
        format!(
            "Failed backing up {} -> {}: {e}",
            original_path.display(),
            backup_path.display()
        )
    })?;

    journal.steps.push(TransactionStep::FileBackup {
        original_path: original_path.to_string_lossy().to_string(),
        backup_path: backup_path.to_string_lossy().to_string(),
        existed: true,
    });
    Ok(Some(backup_path))
}

/// Backs up `target` (if it exists) in `journal`, writes `content` as UTF-8 WITHOUT BOM to a sibling temp file,
/// and atomically replaces `target` via Win32 `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`.
pub fn atomic_write_utf8_no_bom(
    target: &Path,
    content: &str,
    journal: &mut TransactionJournal,
) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed creating parent directory {}: {e}", parent.display()))?;
    }
    let _ = file_backup_journaled(journal, target)?;

    let stamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
    let file_name = target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file.cfg".to_string());
    let temp_path = target.with_file_name(format!(".{file_name}.{stamp}.tmp"));

    // Strip any accidental leading UTF-8 BOM from caller content
    let clean_bytes = content.strip_prefix('\u{FEFF}').unwrap_or(content).as_bytes();
    std::fs::write(&temp_path, clean_bytes)
        .map_err(|e| format!("Failed writing temp file {}: {e}", temp_path.display()))?;

    let w_src = to_wide_null(&temp_path.to_string_lossy());
    let w_dst = to_wide_null(&target.to_string_lossy());
    let move_res = unsafe {
        MoveFileExW(
            PCWSTR(w_src.as_ptr()),
            PCWSTR(w_dst.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if let Err(e) = move_res {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "Atomic MoveFileExW failed for {}: {e}",
            target.display()
        ));
    }
    Ok(())
}

pub fn begin_transaction(title: impl Into<String>) -> TransactionJournal {
    let ts = Local::now();
    let id = format!(
        "TRX_{}_{:04X}",
        ts.format("%Y%m%d_%H%M%S"),
        (ts.timestamp_subsec_millis() & 0xFFFF)
    );
    TransactionJournal {
        journal_id: id,
        title: title.into(),
        timestamp_iso: now_iso(),
        steps: Vec::new(),
        rolled_back: false,
        last_error: None,
    }
}

pub fn commit_transaction(journal: &TransactionJournal) -> Result<Option<PathBuf>, String> {
    if journal.steps.is_empty() {
        return Ok(None);
    }
    ensure_app_dirs();
    let path = journals_dir().join(format!("{}.json", journal.journal_id));
    let json = serde_json::to_string_pretty(journal)
        .map_err(|e| format!("Failed serializing journal: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed saving journal: {e}"))?;
    Ok(Some(path))
}

pub fn save_journal(journal: &TransactionJournal) -> Result<(), String> {
    ensure_app_dirs();
    let path = journals_dir().join(format!("{}.json", journal.journal_id));
    let json = serde_json::to_string_pretty(journal)
        .map_err(|e| format!("Failed serializing journal: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed writing journal: {e}"))
}

pub fn load_all_journals() -> Vec<TransactionJournal> {
    ensure_app_dirs();
    let mut list = Vec::new();
    let Ok(entries) = std::fs::read_dir(journals_dir()) else {
        return list;
    };
    for ent in entries.flatten() {
        let p = ent.path();
        if p.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(raw) = std::fs::read_to_string(&p) {
                if let Ok(j) = serde_json::from_str::<TransactionJournal>(&raw) {
                    list.push(j);
                }
            }
        }
    }
    list.sort_by(|a, b| b.timestamp_iso.cmp(&a.timestamp_iso));
    list
}

pub fn delete_journal_file(journal_id: &str) -> Result<(), String> {
    let path = journals_dir().join(format!("{journal_id}.json"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Delete journal failed: {e}"))?;
    }
    Ok(())
}

/// Rolls back every recorded step in strict LIFO order.
/// Only marks `journal.rolled_back = true` when all steps succeed (`fail_count == 0`).
pub fn rollback_journal(journal: &mut TransactionJournal) -> (usize, usize, Vec<String>) {
    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut errors = Vec::new();

    for step in journal.steps.iter().rev() {
        let step_res: Result<(), String> = match step {
            TransactionStep::RegistryValue {
                hive,
                subkey,
                value_name,
                previous,
                ..
            } => match previous {
                Some(prev_val) => write_typed_reg_value(*hive, subkey, value_name, prev_val),
                None => delete_reg_value(*hive, subkey, value_name),
            },
            TransactionStep::RegistryKeyCreated { hive, subkey } => delete_reg_tree(*hive, subkey),
            TransactionStep::ServiceStartMode {
                service_name,
                previous_start_type,
                ..
            } => set_service_start_raw(service_name, *previous_start_type, false),
            TransactionStep::PowerScheme { previous_guid, .. } => {
                if let Some(g) = parse_guid(previous_guid) {
                    let err = unsafe { PowerSetActiveScheme(None, Some(&g)) };
                    if err == ERROR_SUCCESS {
                        Ok(())
                    } else {
                        Err(format!("PowerSetActiveScheme({previous_guid}) returned {}", err.0))
                    }
                } else {
                    Err(format!("Invalid GUID {previous_guid}"))
                }
            }
            TransactionStep::PowerValueAc {
                scheme_guid,
                subgroup_guid,
                setting_guid,
                previous_val,
                ..
            } => {
                let Some(sg) = parse_guid(scheme_guid) else {
                    errors.push(format!("Invalid scheme GUID {scheme_guid}"));
                    fail += 1;
                    continue;
                };
                let Some(sub) = parse_guid(subgroup_guid) else {
                    errors.push(format!("Invalid subgroup GUID {subgroup_guid}"));
                    fail += 1;
                    continue;
                };
                let Some(set) = parse_guid(setting_guid) else {
                    errors.push(format!("Invalid setting GUID {setting_guid}"));
                    fail += 1;
                    continue;
                };
                unsafe {
                    let err = PowerWriteACValueIndex(None, &sg, Some(&sub), Some(&set), *previous_val);
                    if err == ERROR_SUCCESS {
                        let _ = PowerSetActiveScheme(None, Some(&sg));
                        Ok(())
                    } else {
                        Err(format!("PowerWriteACValueIndex failed: {}", err.0))
                    }
                }
            }
            TransactionStep::FileBackup {
                original_path,
                backup_path,
                existed,
            } => {
                let orig = Path::new(original_path);
                if *existed {
                    let bkp = Path::new(backup_path);
                    if bkp.exists() {
                        std::fs::copy(bkp, orig)
                            .map(|_| ())
                            .map_err(|e| format!("Restore {original_path} failed: {e}"))
                    } else {
                        Err(format!("Backup file missing: {backup_path}"))
                    }
                } else if orig.exists() {
                    std::fs::remove_file(orig)
                        .map_err(|e| format!("Remove created file {original_path} failed: {e}"))
                } else {
                    Ok(())
                }
            }
            TransactionStep::DnsConfig {
                adapter_name,
                previous_servers,
                was_dhcp,
            } => {
                if *was_dhcp || previous_servers.is_empty() {
                    run_command_timeout(
                        "netsh.exe",
                        &[
                            "interface",
                            "ip",
                            "set",
                            "dns",
                            &format!("name=\"{adapter_name}\""),
                            "dhcp",
                        ],
                        5000,
                    )
                    .and_then(|o| {
                        if o.exit_code == 0 {
                            Ok(())
                        } else {
                            Err(format!("netsh reset DNS failed: {}", o.stderr))
                        }
                    })
                } else {
                    let primary = &previous_servers[0];
                    let res1 = run_command_timeout(
                        "netsh.exe",
                        &[
                            "interface",
                            "ip",
                            "set",
                            "dns",
                            &format!("name=\"{adapter_name}\""),
                            "static",
                            primary,
                        ],
                        5000,
                    );
                    match res1 {
                        Ok(o) if o.exit_code == 0 => {
                            if let Some(sec) = previous_servers.get(1) {
                                let _ = run_command_timeout(
                                    "netsh.exe",
                                    &[
                                        "interface",
                                        "ip",
                                        "add",
                                        "dns",
                                        &format!("name=\"{adapter_name}\""),
                                        sec,
                                        "index=2",
                                    ],
                                    5000,
                                );
                            }
                            Ok(())
                        }
                        Ok(o) => Err(format!("netsh restore DNS failed: {}", o.stderr)),
                        Err(e) => Err(e),
                    }
                }
            }
            TransactionStep::NtfsLastAccess { previous_val, .. } => {
                let target_mode = match previous_val {
                    Some(v) => (v & 0x7FFF_FFFF).to_string(),
                    None => "0".to_string(),
                };
                run_command_timeout(
                    "fsutil.exe",
                    &["behavior", "set", "disablelastaccess", &target_mode],
                    5000,
                )
                .and_then(|o| {
                    if o.exit_code == 0 {
                        Ok(())
                    } else {
                        Err(format!("fsutil restore failed: {}", o.stderr))
                    }
                })
            }
        };

        match step_res {
            Ok(()) => ok += 1,
            Err(e) => {
                fail += 1;
                errors.push(e);
            }
        }
    }

    journal.rolled_back = fail == 0;
    journal.last_error = if errors.is_empty() {
        None
    } else {
        Some(errors.join(" | "))
    };
    let _ = save_journal(journal);

    (ok, fail, errors)
}

pub fn desktop_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"))
        .join("Desktop")
}

/// Escapes strings cleanly for cmd.exe `.bat` double-quoted CLI arguments (no injection / `%` expansion).
pub fn escape_bat_double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '%' => out.push_str("%%"),
            '"' => out.push_str("\\\""),
            '\r' | '\n' => {}
            c => out.push(c),
        }
    }
    out
}

pub fn build_emergency_bat_contents(journal: &TransactionJournal) -> String {
    let mut lines = Vec::new();
    lines.push("@echo off".to_string());
    lines.push("setlocal EnableExtensions".to_string());
    lines.push(format!(
        "title Optimizer by Raghav - Emergency Rollback ({})",
        journal.journal_id
    ));
    lines.push("set \"ERR_CNT=0\"".to_string());
    lines.push(format!(
        "echo Rolling back transaction: {}",
        journal.journal_id
    ));
    lines.push("echo.".to_string());

    for step in journal.steps.iter().rev() {
        match step {
            TransactionStep::RegistryValue {
                hive,
                subkey,
                value_name,
                previous,
                ..
            } => {
                let key_path = format!(r"{}\{}", hive.cli_prefix(), subkey);
                let val_flag = if value_name.is_empty() {
                    "/ve".to_string()
                } else {
                    format!("/v \"{}\"", escape_bat_double_quoted(value_name))
                };
                let cmd = match previous {
                    None => format!(
                        "reg delete \"{}\" {} /f >nul 2>&1",
                        escape_bat_double_quoted(&key_path),
                        val_flag
                    ),
                    Some(TypedRegValue::Dword(d)) => format!(
                        "reg add \"{}\" {} /t REG_DWORD /d {} /f >nul 2>&1",
                        escape_bat_double_quoted(&key_path),
                        val_flag,
                        d
                    ),
                    Some(TypedRegValue::Qword(q)) => format!(
                        "reg add \"{}\" {} /t REG_QWORD /d {} /f >nul 2>&1",
                        escape_bat_double_quoted(&key_path),
                        val_flag,
                        q
                    ),
                    Some(TypedRegValue::String(s)) => format!(
                        "reg add \"{}\" {} /t REG_SZ /d \"{}\" /f >nul 2>&1",
                        escape_bat_double_quoted(&key_path),
                        val_flag,
                        escape_bat_double_quoted(s)
                    ),
                    Some(TypedRegValue::ExpandString(s)) => format!(
                        "reg add \"{}\" {} /t REG_EXPAND_SZ /d \"{}\" /f >nul 2>&1",
                        escape_bat_double_quoted(&key_path),
                        val_flag,
                        escape_bat_double_quoted(s)
                    ),
                    Some(TypedRegValue::MultiString(items)) => {
                        let joined = items.join("\\0");
                        format!(
                            "reg add \"{}\" {} /t REG_MULTI_SZ /d \"{}\" /f >nul 2>&1",
                            escape_bat_double_quoted(&key_path),
                            val_flag,
                            escape_bat_double_quoted(&joined)
                        )
                    }
                    Some(TypedRegValue::Binary(bytes)) => {
                        let hex: String = bytes.iter().map(|b| format!("{b:02X}")).collect();
                        format!(
                            "reg add \"{}\" {} /t REG_BINARY /d {} /f >nul 2>&1",
                            escape_bat_double_quoted(&key_path),
                            val_flag,
                            hex
                        )
                    }
                };
                lines.push(cmd);
                lines.push("if errorlevel 1 set /a ERR_CNT+=1".to_string());
            }
            TransactionStep::RegistryKeyCreated { hive, subkey } => {
                let key_path = format!(r"{}\{}", hive.cli_prefix(), subkey);
                lines.push(format!(
                    "reg delete \"{}\" /f >nul 2>&1",
                    escape_bat_double_quoted(&key_path)
                ));
                lines.push("if errorlevel 1 set /a ERR_CNT+=1".to_string());
            }
            TransactionStep::ServiceStartMode {
                service_name,
                previous_start_type,
                ..
            } => {
                let sc_mode = match previous_start_type {
                    2 => "auto",
                    3 => "demand",
                    4 => "disabled",
                    _ => "demand",
                };
                lines.push(format!(
                    "sc config \"{}\" start= {} >nul 2>&1",
                    escape_bat_double_quoted(service_name),
                    sc_mode
                ));
                lines.push("if errorlevel 1 set /a ERR_CNT+=1".to_string());
            }
            TransactionStep::PowerScheme { previous_guid, .. } => {
                lines.push(format!("powercfg /setactive {} >nul 2>&1", previous_guid));
                lines.push("if errorlevel 1 set /a ERR_CNT+=1".to_string());
            }
            TransactionStep::PowerValueAc {
                scheme_guid,
                subgroup_guid,
                setting_guid,
                previous_val,
                ..
            } => {
                lines.push(format!(
                    "powercfg /setacvalueindex {} {} {} {} >nul 2>&1",
                    scheme_guid, subgroup_guid, setting_guid, previous_val
                ));
                lines.push(format!("powercfg /setactive {} >nul 2>&1", scheme_guid));
            }
            TransactionStep::FileBackup {
                original_path,
                backup_path,
                existed,
            } => {
                if *existed && !backup_path.is_empty() {
                    lines.push(format!(
                        "copy /y \"{}\" \"{}\" >nul 2>&1",
                        escape_bat_double_quoted(backup_path),
                        escape_bat_double_quoted(original_path)
                    ));
                    lines.push("if errorlevel 1 set /a ERR_CNT+=1".to_string());
                } else {
                    lines.push(format!(
                        "if exist \"{}\" del /f /q \"{}\" >nul 2>&1",
                        escape_bat_double_quoted(original_path),
                        escape_bat_double_quoted(original_path)
                    ));
                }
            }
            TransactionStep::DnsConfig {
                adapter_name,
                previous_servers,
                was_dhcp,
            } => {
                if *was_dhcp || previous_servers.is_empty() {
                    lines.push(format!(
                        "netsh interface ip set dns name=\"{}\" dhcp >nul 2>&1",
                        escape_bat_double_quoted(adapter_name)
                    ));
                } else {
                    lines.push(format!(
                        "netsh interface ip set dns name=\"{}\" static {} >nul 2>&1",
                        escape_bat_double_quoted(adapter_name),
                        previous_servers[0]
                    ));
                    if let Some(sec) = previous_servers.get(1) {
                        lines.push(format!(
                            "netsh interface ip add dns name=\"{}\" {} index=2 >nul 2>&1",
                            escape_bat_double_quoted(adapter_name),
                            sec
                        ));
                    }
                }
            }
            TransactionStep::NtfsLastAccess { previous_val, .. } => {
                let m = previous_val.map(|v| v & 0x7FFF_FFFF).unwrap_or(0);
                lines.push(format!("fsutil behavior set disablelastaccess {m} >nul 2>&1"));
            }
        }
    }

    lines.push("echo.".to_string());
    lines.push("if \"%ERR_CNT%\"==\"0\" (".to_string());
    lines.push("  echo [OK] Rollback completed with 0 errors.".to_string());
    lines.push(") else (".to_string());
    lines.push("  echo [WARN] Rollback finished with %ERR_CNT% failed step(s).".to_string());
    lines.push(")".to_string());
    lines.push("pause".to_string());
    lines.join("\r\n")
}

pub fn export_emergency_bat(journal: &TransactionJournal) -> Result<PathBuf, String> {
    let dir = desktop_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("OptimizerByRaghav_Undo_{}.bat", journal.journal_id));
    let content = build_emergency_bat_contents(journal);
    std::fs::write(&path, content).map_err(|e| format!("Failed writing batch file: {e}"))?;
    Ok(path)
}

pub fn check_vss_health() -> VssHealth {
    let sys_drive = system_drive_root();
    let wide_drive = to_wide_null(&sys_drive);
    let mut free_avail = 0u64;
    unsafe {
        let _ = GetDiskFreeSpaceExW(PCWSTR(wide_drive.as_ptr()), Some(&mut free_avail), None, None);
    }
    let sufficient_space = free_avail >= 5 * 1024 * 1024 * 1024;

    let (service_exists, start_type, status_text, is_disabled) =
        match service_query_state("VSS") {
            Ok((st, state)) => (true, st, state, st == 4),
            Err(_) => (false, 4, "Not Found".to_string(), true),
        };

    // Check System Restore policy & RPSessionInterval / DisableSR
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let restore_disabled_policy = hklm
        .open_subkey_with_flags(
            r"SOFTWARE\Policies\Microsoft\Windows NT\SystemRestore",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .and_then(|k| k.get_value::<u32, _>("DisableSR"))
        .map(|v| v == 1)
        .unwrap_or(false);
    let restore_disabled_rt = hklm
        .open_subkey_with_flags(SYSTEM_RESTORE_SUBKEY, KEY_READ | KEY_WOW64_64KEY)
        .and_then(|k| k.get_value::<u32, _>("DisableSR"))
        .map(|v| v == 1)
        .unwrap_or(false);
    let restore_enabled = Some(!(restore_disabled_policy || restore_disabled_rt));

    let start_label = match start_type {
        2 => "Auto",
        3 => "Manual",
        4 => "Disabled",
        _ => "Other",
    };

    let summary = if !service_exists {
        "Volume Shadow Copy service (VSS) not available".to_string()
    } else if is_disabled {
        format!(
            "VSS service is Disabled ({} free on {}) — Restore Points will fail unless enabled",
            format_bytes(free_avail),
            sys_drive
        )
    } else if !sufficient_space {
        format!(
            "Low free space on {} ({}) — keep >= 5 GB free for Restore Points",
            sys_drive,
            format_bytes(free_avail)
        )
    } else if restore_enabled == Some(false) {
        "Windows System Protection is disabled by policy (DisableSR=1)".to_string()
    } else {
        format!(
            "VSS Ready (Start: {start_label}, State: {status_text}, {} free on {sys_drive})",
            format_bytes(free_avail)
        )
    };

    VssHealth {
        service_exists,
        start_type,
        status_text,
        is_disabled,
        system_drive: sys_drive,
        system_drive_free_bytes: free_avail,
        sufficient_space,
        restore_enabled,
        summary,
    }
}

pub fn list_restore_points() -> Vec<RestorePointInfo> {
    let mut points = Vec::new();
    let Ok(com) = COMLibrary::new() else {
        return points;
    };
    let Ok(con) = WMIConnection::with_namespace_path(r"ROOT\DEFAULT", com) else {
        return points;
    };
    let Ok(rows) = con.raw_query::<HashMap<String, Variant>>(
        "SELECT SequenceNumber, Description, CreationTime, RestorePointType FROM SystemRestore",
    ) else {
        return points;
    };

    for row in rows {
        let seq = var_u32(&row, "SequenceNumber").unwrap_or(0);
        let desc = var_str(&row, "Description").unwrap_or_else(|| "System Restore Point".to_string());
        let raw_time = var_str(&row, "CreationTime").unwrap_or_default();
        let created_at = format_dmtf_datetime(&raw_time);
        let rtype = match var_u32(&row, "RestorePointType").unwrap_or(0) {
            0 => "Application Install",
            1 => "Application Uninstall",
            10 => "Device Driver Install",
            12 => "Modify Settings",
            _ => "System Checkpoint",
        }
        .to_string();

        points.push(RestorePointInfo {
            sequence_number: seq,
            description: desc,
            created_at,
            restore_type: rtype,
        });
    }

    points.sort_by(|a, b| b.sequence_number.cmp(&a.sequence_number));
    points
}

pub fn create_restore_point(description: &str) -> Result<(), String> {
    // Capture & temporarily override SystemRestorePointCreationFrequency, then ALWAYS restore
    let prev_freq = read_typed_reg_value(RegHive::Hklm, SYSTEM_RESTORE_SUBKEY, SR_FREQ_VALUE);
    let _ = write_typed_reg_value(
        RegHive::Hklm,
        SYSTEM_RESTORE_SUBKEY,
        SR_FREQ_VALUE,
        &TypedRegValue::Dword(0),
    );

    let sanitized: String = description
        .chars()
        .filter(|c| !matches!(c, '\'' | '"' | '`' | '$'))
        .collect();
    let script = format!(
        "Checkpoint-Computer -Description '{}' -RestorePointType 'MODIFY_SETTINGS' -ErrorAction Stop",
        sanitized
    );

    let res = run_powershell_encoded(&script, 45_000);

    // Always restore original frequency policy
    match prev_freq {
        Some(v) => {
            let _ = write_typed_reg_value(RegHive::Hklm, SYSTEM_RESTORE_SUBKEY, SR_FREQ_VALUE, &v);
        }
        None => {
            let _ = delete_reg_value(RegHive::Hklm, SYSTEM_RESTORE_SUBKEY, SR_FREQ_VALUE);
        }
    }

    match res {
        Ok(out) if out.exit_code == 0 => Ok(()),
        Ok(out) => Err(format!(
            "Checkpoint-Computer failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        )),
        Err(e) => Err(e),
    }
}

pub fn launch_system_restore_ui() -> Result<(), String> {
    Command::new("rstrui.exe")
        .creation_flags(0)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch rstrui.exe: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_reg_value_roundtrip_tests() {
        let cases = [
            TypedRegValue::Dword(0),
            TypedRegValue::Dword(u32::MAX), // previously collided with -1 sentinel!
            TypedRegValue::Qword(0xDEAD_BEEF_CAFE_BABE),
            TypedRegValue::String("C:\\Program Files\\App\\app.exe".to_string()),
            TypedRegValue::ExpandString("%SystemRoot%\\System32".to_string()),
            TypedRegValue::MultiString(vec!["line1".into(), "line2".into()]),
            TypedRegValue::Binary(vec![0x00, 0x1A, 0xFF, 0x42]),
        ];

        for case in &cases {
            let raw = typed_to_raw(case);
            let decoded = raw_to_typed(&raw).expect("decode failed");
            assert_eq!(case, &decoded);
        }
    }

    #[test]
    fn emergency_bat_escaping_tests() {
        assert_eq!(escape_bat_double_quoted("%PATH%"), "%%PATH%%");
        assert_eq!(escape_bat_double_quoted("a\"b"), "a\\\"b");

        let mut j = begin_transaction("Test Batch Export");
        j.steps.push(TransactionStep::RegistryValue {
            hive: RegHive::Hkcu,
            subkey: r"Environment".to_string(),
            value_name: "DemoPath".to_string(),
            previous: Some(TypedRegValue::ExpandString(r"%SystemRoot%\System32".to_string())),
            applied: None,
        });
        j.steps.push(TransactionStep::RegistryValue {
            hive: RegHive::Hklm,
            subkey: r"SYSTEM\CurrentControlSet\Services\MM".to_string(),
            value_name: "NetworkThrottlingIndex".to_string(),
            previous: Some(TypedRegValue::Dword(u32::MAX)),
            applied: None,
        });

        let bat = build_emergency_bat_contents(&j);
        assert!(bat.contains("%%SystemRoot%%\\System32"));
        assert!(bat.contains("4294967295"));
        assert!(bat.contains("ERR_CNT"));
    }
}
