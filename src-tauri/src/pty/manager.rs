use std::io::{Read, Write};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

const MAX_PTYS: usize = 16;

pub struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

/// Ids become part of an event name, so restrict them to a safe charset.
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn size(rows: u16, cols: u16) -> PtySize {
    PtySize { rows: rows.max(1), cols: cols.max(1), pixel_width: 0, pixel_height: 0 }
}

/// Split off any trailing incomplete UTF-8 sequence so multi-byte chars are never cut across reads.
fn split_complete_utf8(buf: &[u8]) -> usize {
    match std::str::from_utf8(buf) {
        Ok(_) => buf.len(),
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        Err(_) => buf.len(), // genuinely invalid bytes: let from_utf8_lossy replace them
    }
}

#[tauri::command]
pub fn create_pty(app: AppHandle, state: State<'_, AppState>, id: String, rows: u16, cols: u16) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("invalid pty id".into());
    }
    let mut ptys = state.ptys.lock().map_err(|e| e.to_string())?;
    if ptys.contains_key(&id) || ptys.len() >= MAX_PTYS {
        return Err("pty already exists or limit reached".into());
    }

    let pair = native_pty_system().openpty(size(rows, cols)).map_err(|e| e.to_string())?;
    let child = pair.slave.spawn_command(CommandBuilder::new_default_prog()).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let event = format!("pty-output:{id}");
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            match reader.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    pending.extend_from_slice(&chunk[..n]);
                    let cut = split_complete_utf8(&pending);
                    let text = String::from_utf8_lossy(&pending[..cut]).into_owned();
                    pending.drain(..cut);
                    if !text.is_empty() && app.emit(&event, text).is_err() {
                        break;
                    }
                }
            }
        }
    });

    ptys.insert(id, PtyInstance { master: pair.master, writer, child });
    Ok(())
}

#[tauri::command]
pub fn write_pty(state: State<'_, AppState>, id: String, data: String) -> Result<(), String> {
    let mut ptys = state.ptys.lock().map_err(|e| e.to_string())?;
    let p = ptys.get_mut(&id).ok_or("no such pty")?;
    p.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resize_pty(state: State<'_, AppState>, id: String, rows: u16, cols: u16) -> Result<(), String> {
    let ptys = state.ptys.lock().map_err(|e| e.to_string())?;
    let p = ptys.get(&id).ok_or("no such pty")?;
    p.master.resize(size(rows, cols)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_pty(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut ptys = state.ptys.lock().map_err(|e| e.to_string())?;
    if let Some(mut p) = ptys.remove(&id) {
        let _ = p.child.kill();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_boundary_is_held_back() {
        let euro = "€".as_bytes(); // 3 bytes
        assert_eq!(split_complete_utf8(&euro[..2]), 0);
        assert_eq!(split_complete_utf8(b"ab\xe2\x82"), 2);
        assert_eq!(split_complete_utf8(euro), 3);
    }

    #[test]
    fn id_validation() {
        assert!(valid_id("pty-1"));
        assert!(!valid_id("a b"));
        assert!(!valid_id(""));
        assert!(!valid_id("x:y"));
    }
}
