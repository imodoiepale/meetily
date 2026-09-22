// Deepgram HTTP listen (chunked, not the OpenWhispr WebSocket streaming client).
//
// Auth header shape from vendor/openwhispr/src/helpers/deepgramStreaming.js:
// BYOK keys use `Token {key}`. Default model nova-2 (nova-3 when selected).

use super::pcm_wav::{f32_pcm_to_wav, MIN_CLOUD_SAMPLES};
use super::provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
use async_trait::async_trait;
use log::{info, warn};
use serde::Deserialize;

const DEFAULT_MODEL: &str = "nova-2";

#[derive(Deserialize)]
struct DeepgramResponse {
    results: Option<DeepgramResults>,
}

#[derive(Deserialize)]
struct DeepgramResults {
    channels: Option<Vec<DeepgramChannel>>,
}

#[derive(Deserialize)]
struct DeepgramChannel {
    alternatives: Option<Vec<DeepgramAlternative>>,
}

#[derive(Deserialize)]
struct DeepgramAlternative {
    transcript: Option<String>,
    confidence: Option<f32>,
}

pub struct DeepgramProvider {
    api_key: String,
    model: String,
}

impl DeepgramProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let model = if model.trim().is_empty() || !model.starts_with("nova-") {
            DEFAULT_MODEL.to_string()
        } else {
            model
        };
        Self { api_key, model }
    }
}

#[async_trait]
impl TranscriptionProvider for DeepgramProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError> {
        if self.api_key.trim().is_empty() {
            return Err(TranscriptionError::ModelNotLoaded);
        }
        if audio.len() < MIN_CLOUD_SAMPLES {
            return Err(TranscriptionError::AudioTooShort {
                samples: audio.len(),
                minimum: MIN_CLOUD_SAMPLES,
            });
        }

        let wav = f32_pcm_to_wav(&audio, 16_000);
        let mut url = format!(
            "https://api.deepgram.com/v1/listen?model={}&smart_format=true&punctuate=true",
            urlencoding_model(&self.model)
        );
        if let Some(lang) = language {
            let code = lang
                .split(['-', '_'])
                .next()
                .unwrap_or("en")
                .to_lowercase();
            if code != "auto" && !code.is_empty() {
                url.push_str("&language=");
                url.push_str(&code);
            }
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_key.trim()))
            .header("Content-Type", "audio/wav")
            .timeout(std::time::Duration::from_secs(30))
            .body(wav)
            .send()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(format!("Deepgram request failed: {e}")))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?;

        if !status.is_success() {
            warn!("Deepgram STT HTTP {}: {}", status, body);
            return Err(TranscriptionError::EngineFailed(format!(
                "Deepgram transcription failed ({status}): {body}"
            )));
        }

        let parsed: DeepgramResponse = serde_json::from_str(&body).map_err(|e| {
            TranscriptionError::EngineFailed(format!("Deepgram response parse error: {e}"))
        })?;
        let alt = parsed
            .results
            .and_then(|r| r.channels)
            .and_then(|chs| chs.into_iter().next())
            .and_then(|ch| ch.alternatives)
            .and_then(|alts| alts.into_iter().next());

        let text = alt
            .as_ref()
            .and_then(|a| a.transcript.clone())
            .unwrap_or_default()
            .trim()
            .to_string();
        let confidence = alt.and_then(|a| a.confidence);
        info!("Deepgram transcribed {} chars", text.len());
        Ok(TranscriptResult {
            text,
            confidence,
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Deepgram"
    }
}

fn urlencoding_model(model: &str) -> String {
    model
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}
