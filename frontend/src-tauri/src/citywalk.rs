// CityWalk Task Manager ingest: signed POST of a local meeting's transcript
// and minutes. Config lives in the app data dir as citywalk.json so it never
// rides the Meetily settings schema.

use crate::database::repositories::{
    meeting::MeetingsRepository, summary::SummaryProcessesRepository,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CityWalkConfig {
    pub ingest_url: String,
    pub ingest_secret: String,
    #[serde(default)]
    pub meeting_type: String,
}

fn config_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(dir.join("citywalk.json"))
}

fn load_config<R: Runtime>(app: &AppHandle<R>) -> Result<CityWalkConfig, String> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(CityWalkConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid citywalk.json: {e}"))
}

fn save_config<R: Runtime>(app: &AppHandle<R>, config: &CityWalkConfig) -> Result<(), String> {
    let path = config_path(app)?;
    let raw = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn citywalk_get_config<R: Runtime>(app: AppHandle<R>) -> Result<CityWalkConfig, String> {
    load_config(&app)
}

#[tauri::command]
pub async fn citywalk_save_config<R: Runtime>(
    app: AppHandle<R>,
    ingest_url: String,
    ingest_secret: String,
    meeting_type: Option<String>,
) -> Result<CityWalkConfig, String> {
    let config = CityWalkConfig {
        ingest_url: ingest_url.trim().to_string(),
        ingest_secret: ingest_secret.trim().to_string(),
        meeting_type: meeting_type.unwrap_or_default(),
    };
    save_config(&app, &config)?;
    Ok(config)
}

#[derive(Serialize)]
struct IngestTranscript {
    text: String,
    timestamp: String,
    audio_start_time: Option<f64>,
    audio_end_time: Option<f64>,
}

#[tauri::command]
pub async fn citywalk_publish_meeting<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    meeting_type: Option<String>,
    department_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let config = load_config(&app)?;
    if config.ingest_url.trim().is_empty() || config.ingest_secret.trim().is_empty() {
        return Err(
            "CityWalk ingest URL and secret are not set. Add them in Preferences.".to_string(),
        );
    }

    let pool = state.db_manager.pool();
    let meeting = MeetingsRepository::get_meeting(pool, &meeting_id)
        .await
        .map_err(|e| format!("Meeting {meeting_id} not found: {e}"))?;

    let minutes_process =
        SummaryProcessesRepository::get_summary_data_for_meeting(pool, &meeting_id)
            .await
            .map_err(|e| e.to_string())?;

    let minutes = minutes_process
        .as_ref()
        .and_then(|p| p.result.as_ref())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());

    let minutes_markdown = minutes.as_ref().and_then(extract_markdown);

    let payload = serde_json::json!({
        "desktop_meeting_id": meeting_id,
        "title": meeting.title,
        "meeting_type": meeting_type
            .filter(|s| !s.is_empty())
            .or_else(|| {
                if config.meeting_type.is_empty() {
                    None
                } else {
                    Some(config.meeting_type.clone())
                }
            })
            .unwrap_or_else(|| "other".to_string()),
        "department_id": department_id,
        "started_at": meeting.created_at,
        "transcript": meeting
            .transcripts
            .into_iter()
            .map(|t| IngestTranscript {
                text: t.text,
                timestamp: t.timestamp,
                audio_start_time: t.audio_start_time,
                audio_end_time: t.audio_end_time,
            })
            .collect::<Vec<_>>(),
        "minutes": minutes,
        "minutes_markdown": minutes_markdown,
        "source": "desktop",
    });

    let url = config.ingest_url.trim().trim_end_matches('/').to_string();
    let ingest_url = if url.ends_with("/ingest") {
        url
    } else {
        format!("{url}/api/integrations/meetings/ingest")
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&ingest_url)
        .header("X-CityWalk-Ingest-Secret", config.ingest_secret.trim())
        .header("Content-Type", "application/json")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Could not reach CityWalk: {e}"))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("CityWalk ingest failed ({status}): {body}"));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|_| serde_json::json!({ "raw": body }));
    Ok(parsed)
}

fn extract_markdown(value: &serde_json::Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    if let Some(s) = value.get("markdown").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    if let Some(s) = value.get("summary").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    serde_json::to_string_pretty(value).ok()
}
