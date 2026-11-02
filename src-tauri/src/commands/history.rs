use crate::managers::{
    history::{HistoryEntry, HistoryManager, PaginatedHistory},
    transcription_jobs::TranscriptionJobManager,
};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<PaginatedHistory, String> {
    history_manager
        .get_history_entries(cursor, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_history_entry_saved(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .toggle_saved_status(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_audio_file_path(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_name: String,
) -> Result<String, String> {
    // Security hardening: only a bare recording file name is accepted, so the
    // webview cannot resolve paths outside the recordings folder ("..\..\x").
    if !is_bare_file_name(&file_name) {
        return Err("Invalid recording file name".to_string());
    }
    let path = history_manager.get_audio_file_path(&file_name);
    if !path.is_file() {
        return Err(
            "This recording's audio is no longer kept (auto-wipe); its text is".to_string(),
        );
    }
    path.to_str()
        .ok_or_else(|| "Invalid file path".to_string())
        .map(|s| s.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    jobs: State<'_, Arc<TranscriptionJobManager>>,
    id: i64,
) -> Result<(), String> {
    jobs.cancel(id);
    history_manager
        .delete_entry(id)
        .await
        .map_err(|e| e.to_string())
}

/// Queues the entry to be transcribed again with the current model. Its
/// progress and outcome arrive as `TranscriptionJob` events.
#[tauri::command]
#[specta::specta]
pub fn retry_history_entry_transcription(jobs: State<'_, Arc<TranscriptionJobManager>>, id: i64) {
    jobs.retry(id);
}

/// Stops the entry's background transcription: a queued one is dropped, a
/// running one stops after the chunk it is on. The entry keeps the text it
/// had before.
#[tauri::command]
#[specta::specta]
pub fn cancel_history_entry_transcription(jobs: State<'_, Arc<TranscriptionJobManager>>, id: i64) {
    jobs.cancel(id);
}

/// Adds an audio or video file to History and queues its transcription.
/// Fails without adding anything if the file has no audio Handy can read.
#[tauri::command]
#[specta::specta]
pub async fn transcribe_file(
    jobs: State<'_, Arc<TranscriptionJobManager>>,
    path: String,
) -> Result<HistoryEntry, String> {
    let jobs = Arc::clone(&jobs);
    tauri::async_runtime::spawn_blocking(move || jobs.transcribe_file(PathBuf::from(path)))
        .await
        .map_err(|e| format!("Failed to read the file: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_history_limit(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    limit: usize,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.history_limit = limit;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_recording_retention_period(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    period: String,
) -> Result<(), String> {
    use crate::settings::RecordingRetentionPeriod;

    let retention_period = match period.as_str() {
        "never" => RecordingRetentionPeriod::Never,
        "preserve_limit" => RecordingRetentionPeriod::PreserveLimit,
        "days3" => RecordingRetentionPeriod::Days3,
        "weeks2" => RecordingRetentionPeriod::Weeks2,
        "months3" => RecordingRetentionPeriod::Months3,
        _ => return Err(format!("Invalid retention period: {}", period)),
    };

    let mut settings = crate::settings::get_settings(&app);
    settings.recording_retention_period = retention_period;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// True for a plain file name: no separators, no parent/current-dir parts,
/// no drive or stream syntax (`C:`, `file:stream`), and no control characters.
fn is_bare_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':', '\0'])
        && !name.chars().any(char::is_control)
        && std::path::Path::new(name).file_name() == Some(std::ffi::OsStr::new(name))
}

#[cfg(test)]
mod bare_file_name_tests {
    use super::is_bare_file_name;

    #[test]
    fn accepts_recording_names() {
        assert!(is_bare_file_name("handy-1730000000.wav"));
        assert!(is_bare_file_name("recording 2026-11-02.wav"));
    }

    #[test]
    fn rejects_paths_and_traversal() {
        for bad in [
            "",
            ".",
            "..",
            "../secret",
            "..\\..\\Windows\\win.ini",
            "/etc/passwd",
            "C:\\Users\\x\\.ssh\\id_rsa",
            "a/b.wav",
            "a\\b.wav",
            "C:x.wav",
            "file.wav:stream",
            "bad\0.wav",
            "bad\n.wav",
        ] {
            assert!(!is_bare_file_name(bad), "should reject {bad:?}");
        }
    }
}
