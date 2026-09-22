// Groq Whisper HTTP transcription.
//
// Ported from OpenWhispr's OpenAI-compatible multipart STT path
// (vendor/openwhispr/src/helpers/audioManager.js + transcriptionRoute.ts):
// POST https://api.groq.com/openai/v1/audio/transcriptions
// default model whisper-large-v3-turbo.

use super::pcm_wav::{f32_pcm_to_wav, MIN_CLOUD_SAMPLES};
use super::provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
use async_trait::async_trait;
use log::{info, warn};
use serde::Deserialize;

const GROQ_TRANSCRIBE_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const DEFAULT_MODEL: &str = "whisper-large-v3-turbo";

#[derive(Deserialize)]
struct GroqTranscriptionResponse {
    text: Option<String>,
}

pub struct GroqWhisperProvider {
    api_key: String,
    model: String,
}

impl GroqWhisperProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let model = if model.trim().is_empty() || !model.starts_with("whisper-") {
            DEFAULT_MODEL.to_string()
        } else {
            model
        };
        Self { api_key, model }
    }
}

#[async_trait]
impl TranscriptionProvider for GroqWhisperProvider {
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
        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("chunk.wav")
            .mime_str("audio/wav")
            .map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json")
            .text("temperature", "0");

        if let Some(lang) = language {
            let code = lang
                .split(['-', '_'])
                .next()
                .unwrap_or("en")
                .to_lowercase();
            if code != "auto" && !code.is_empty() {
                form = form.text("language", code);
            }
        }

        let client = reqwest::Client::new();
        let response = client
            .post(GROQ_TRANSCRIBE_URL)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(format!("Groq request failed: {e}")))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?;

        if !status.is_success() {
            warn!("Groq STT HTTP {}: {}", status, body);
            return Err(TranscriptionError::EngineFailed(format!(
                "Groq transcription failed ({status}): {body}"
            )));
        }

        let parsed: GroqTranscriptionResponse = serde_json::from_str(&body).map_err(|e| {
            TranscriptionError::EngineFailed(format!("Groq response parse error: {e}"))
        })?;
        let text = parsed.text.unwrap_or_default().trim().to_string();
        info!("Groq Whisper transcribed {} chars", text.len());
        Ok(TranscriptResult {
            text,
            confidence: None,
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
        "Groq Whisper"
    }
}
