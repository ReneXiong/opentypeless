use async_trait::async_trait;
use base64::Engine;
use futures_util::StreamExt;

use crate::error::AppError;
use crate::llm::{ChunkCallback, LlmConfig};

use super::{MultimodalProvider, MultimodalRequest, MultimodalResponse};

pub struct OpenAiMultimodalProvider {
    client: reqwest::Client,
}

impl Default for OpenAiMultimodalProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAiMultimodalProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl MultimodalProvider for OpenAiMultimodalProvider {
    async fn process(
        &self,
        config: &LlmConfig,
        req: &MultimodalRequest,
        on_chunk: Option<&ChunkCallback>,
    ) -> Result<MultimodalResponse, AppError> {
        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&req.audio_wav);

        let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "system",
            "content": req.system_prompt
        })];

        // If selected text is present, add it as a separate user message first
        if let Some(ref selected) = req.selected_text {
            messages.push(serde_json::json!({
                "role": "user",
                "content": format!("<selected_text>{}</selected_text>", selected)
            }));
        }

        // Audio message with input_audio content type
        messages.push(serde_json::json!({
            "role": "user",
            "content": [
                {
                    "type": "input_audio",
                    "input_audio": {
                        "data": audio_b64,
                        "format": "wav"
                    }
                }
            ]
        }));

        let stream = on_chunk.is_some();
        let mut body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": stream,
        });

        // Apply reasoning_effort for OpenAI-compatible APIs
        if !config.reasoning_effort.is_empty() {
            let effort = if config.reasoning_effort == "off" {
                if config.base_url.contains("openrouter") {
                    "none".to_string()
                } else {
                    "low".to_string()
                }
            } else {
                config.reasoning_effort.clone()
            };
            body["reasoning_effort"] = serde_json::json!(effort);
        }

        // GLM thinking mode support (skip for "off" or "low" effort)
        if config.model.starts_with("glm-") && config.reasoning_effort != "off" && config.reasoning_effort != "low" {
            body["thinking"] = serde_json::json!({"type": "enabled"});
            body["temperature"] = serde_json::json!(1.0);
            body["top_p"] = serde_json::json!(0.95);
        }

        let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

        let mut attempt = 0u32;
        loop {
            let resp_result = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .timeout(std::time::Duration::from_secs(120))
                .json(&body)
                .send()
                .await;

            match resp_result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        if stream {
                            return self.handle_streaming(resp, on_chunk).await;
                        } else {
                            return self.handle_non_streaming(resp).await;
                        }
                    } else if status.as_u16() >= 500 && attempt < 2 {
                        tracing::warn!(
                            "Multimodal API error {} (attempt {}/3)",
                            status,
                            attempt + 1
                        );
                        attempt += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(
                            1000 * 2u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    } else {
                        let body_text = resp.text().await.unwrap_or_default();
                        let truncated = truncate_str(&body_text, 200);
                        tracing::error!("Multimodal API error {}: {}", status, truncated);
                        return Err(AppError::Api {
                            status: status.as_u16(),
                            body: truncated.to_string(),
                        });
                    }
                }
                Err(e) if e.is_timeout() && attempt < 2 => {
                    tracing::warn!("Multimodal API timeout (attempt {}/3)", attempt + 1);
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
        "OpenAI Multimodal"
    }
}

impl OpenAiMultimodalProvider {
    async fn handle_streaming(
        &self,
        resp: reqwest::Response,
        on_chunk: Option<&ChunkCallback>,
    ) -> Result<MultimodalResponse, AppError> {
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();
        let mut reasoning_text = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| AppError::Network(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }
                let data = line[5..].trim();
                if data == "[DONE]" {
                    continue;
                }

                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    let delta = &v["choices"][0]["delta"];

                    // Regular content
                    if let Some(content) = delta["content"].as_str() {
                        full_text.push_str(content);
                        if let Some(cb) = on_chunk {
                            cb(content);
                        }
                    }

                    // Reasoning content (GLM thinking models)
                    if let Some(reasoning) = delta["reasoning_content"].as_str() {
                        reasoning_text.push_str(reasoning);
                    }
                }
            }
        }

        // GLM fallback: if no content but reasoning exists, use reasoning
        if full_text.is_empty() && !reasoning_text.is_empty() {
            full_text = reasoning_text;
        }

        if full_text.is_empty() {
            return Err(AppError::Config(
                "Multimodal API returned empty response".to_string(),
            ));
        }

        Ok(MultimodalResponse { text: full_text })
    }

    async fn handle_non_streaming(
        &self,
        resp: reqwest::Response,
    ) -> Result<MultimodalResponse, AppError> {
        let body = resp.text().await.map_err(|e| AppError::Network(e.to_string()))?;
        let v: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| AppError::Config(e.to_string()))?;

        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if text.is_empty() {
            return Err(AppError::Config(
                "Multimodal API returned empty response".to_string(),
            ));
        }

        Ok(MultimodalResponse { text })
    }
}

fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
