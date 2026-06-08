use async_trait::async_trait;
use base64::Engine;

use crate::error::AppError;

use super::{SttConfig, SttProvider, TranscriptEvent};

/// Max audio buffer: ~24 MB PCM ≈ 12.5 min at 16kHz 16-bit mono.
const MAX_AUDIO_BYTES: usize = 24 * 1024 * 1024;

/// MiMo-V2.5-ASR provider using OpenAI-compatible chat completions endpoint.
/// API: https://api.xiaomimimo.com/v1/chat/completions
/// Model: mimo-v2.5-asr
pub struct MimoAsrProvider {
    stt_config: Option<SttConfig>,
    audio_buffer: Vec<u8>,
    client: reqwest::Client,
}

impl MimoAsrProvider {
    pub fn new() -> Self {
        Self {
            stt_config: None,
            audio_buffer: Vec::new(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            stt_config: None,
            audio_buffer: Vec::new(),
            client,
        }
    }

    /// Build a WAV file from raw PCM 16-bit mono audio.
    fn build_wav(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
        let data_len = pcm.len() as u32;
        let channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * (channels as u32) * (bits_per_sample as u32) / 8;
        let block_align = channels * bits_per_sample / 8;
        let file_size = 36 + data_len;

        let mut wav = Vec::with_capacity(44 + pcm.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(pcm);
        wav
    }
}

#[async_trait]
impl SttProvider for MimoAsrProvider {
    async fn connect(&mut self, config: &SttConfig) -> Result<(), AppError> {
        if config.api_key.is_empty() {
            return Err(AppError::Auth("MiMo ASR API key is empty".to_string()));
        }
        self.stt_config = Some(config.clone());
        self.audio_buffer.clear();
        tracing::info!("MiMo ASR provider ready (buffering mode)");
        Ok(())
    }

    async fn send_audio(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        if self.audio_buffer.len() + chunk.len() > MAX_AUDIO_BYTES {
            return Err(AppError::Config(
                "MiMo ASR: audio exceeds maximum length (~12 min)".to_string(),
            ));
        }
        self.audio_buffer.extend_from_slice(chunk);
        Ok(())
    }

    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> {
        // File-based — transcription happens in disconnect().
        Ok(None)
    }

    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        let config = match &self.stt_config {
            Some(c) => c.clone(),
            None => return Ok(None),
        };

        if self.audio_buffer.is_empty() {
            tracing::info!("MiMo ASR: no audio buffered, skipping");
            return Ok(None);
        }

        let audio_len_secs = self.audio_buffer.len() as f64 / (config.sample_rate as f64 * 2.0);
        let wav_data = Self::build_wav(&self.audio_buffer, config.sample_rate);
        self.audio_buffer.clear();
        tracing::info!(
            "MiMo ASR: sending {:.1}s of audio for transcription",
            audio_len_secs
        );

        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&wav_data);
        // MiMo ASR requires data URL format: data:audio/wav;base64,$BASE64
        let audio_data_url = format!("data:audio/wav;base64,{}", audio_b64);

        // MiMo ASR API: only audio part allowed, no text prompt
        // Language is specified via asr_options
        let messages = serde_json::json!([
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": audio_data_url
                        }
                    }
                ]
            }
        ]);

        // Build language option
        let language = match &config.language {
            Some(lang) if lang != "multi" => lang.as_str(),
            _ => "auto",
        };

        let body = serde_json::json!({
            "model": "mimo-v2.5-asr",
            "messages": messages,
            "asr_options": {
                "language": language
            },
            "stream": false
        });

        let mut attempt = 0u32;
        loop {
            let resp_result = self
                .client
                .post("https://api.xiaomimimo.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .timeout(std::time::Duration::from_secs(120))
                .send()
                .await;

            match resp_result {
                Ok(resp) => {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();

                    if status.is_success() {
                        let v: serde_json::Value = serde_json::from_str(&body_text)
                            .map_err(|e| AppError::Config(e.to_string()))?;

                        // Extract text from chat completion response
                        let text = v["choices"][0]["message"]["content"]
                            .as_str()
                            .unwrap_or("")
                            .trim()
                            .to_string();

                        // Remove language tags if present (e.g., <chinese>)
                        let text = if text.starts_with('<') {
                            if let Some(end) = text.find('>') {
                                text[end + 1..].trim().to_string()
                            } else {
                                text
                            }
                        } else {
                            text
                        };

                        tracing::info!("MiMo ASR transcription: {} chars", text.len());
                        return Ok(if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        });
                    } else if status.as_u16() >= 500 && attempt < 2 {
                        let truncate_at = body_text
                            .char_indices()
                            .take_while(|&(i, _)| i < 200)
                            .last()
                            .map(|(i, c)| i + c.len_utf8())
                            .unwrap_or(body_text.len());
                        tracing::warn!(
                            "MiMo ASR server error {} (attempt {}/3): {}",
                            status,
                            attempt + 1,
                            &body_text[..truncate_at]
                        );
                        attempt += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(
                            1000 * 2u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    } else if status.as_u16() == 429 {
                        // Rate limit error - provide more specific message
                        tracing::error!("MiMo ASR rate limit exceeded (429). Check your API quota at https://platform.xiaomimimo.com/console/balance");
                        return Err(AppError::Api {
                            status: 429,
                            body: "Rate limit exceeded. Please check your API quota or try again later.".to_string(),
                        });
                    } else {
                        let truncate_at = body_text
                            .char_indices()
                            .take_while(|&(i, _)| i < 200)
                            .last()
                            .map(|(i, c)| i + c.len_utf8())
                            .unwrap_or(body_text.len());
                        let sanitized = &body_text[..truncate_at];
                        tracing::error!("MiMo ASR HTTP {}: {}", status, sanitized);
                        return Err(AppError::Api {
                            status: status.as_u16(),
                            body: sanitized.to_string(),
                        });
                    }
                }
                Err(e) if e.is_timeout() && attempt < 2 => {
                    tracing::warn!(
                        "MiMo ASR timeout (attempt {}/3)",
                        attempt + 1
                    );
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(
                        1000 * 2u64.pow(attempt - 1),
                    ))
                    .await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn name(&self) -> &str {
        "mimo-asr"
    }
}
