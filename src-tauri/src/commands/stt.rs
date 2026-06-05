use crate::api_base_url;
use crate::stt;
use crate::SessionTokenStore;

/// Build and send a test request for the given STT provider.
/// Returns Ok(response) on success, Err on request failure.
async fn send_stt_test_request(
    provider: &str,
    api_key: &str,
    client: &reqwest::Client,
    token_store: &SessionTokenStore,
) -> Result<reqwest::Response, String> {
    match provider {
        "cloud" => {
            let token = token_store
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if token.is_empty() {
                return Err("Not signed in".to_string());
            }
            let api_base = api_base_url();
            client
                .get(format!("{}/api/subscription/status", api_base))
                .header("Authorization", format!("Bearer {}", token))
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())
        }
        "deepgram" => {
            client
                .get("https://api.deepgram.com/v1/projects")
                .header("Authorization", format!("Token {}", api_key))
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())
        }
        "assemblyai" => {
            client
                .get("https://api.assemblyai.com/v2/transcript?limit=1")
                .header("Authorization", api_key)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())
        }
        _ => {
            let cfg = stt::config::get_whisper_config(provider)
                .ok_or_else(|| format!("Unknown STT provider: {}", provider))?;

            let silent_pcm = vec![0u8; 3200]; // 0.1s at 16kHz 16-bit mono
            let wav = stt::whisper_compat::WhisperCompatProvider::build_wav(&silent_pcm, 16000);

            let file_part = reqwest::multipart::Part::bytes(wav)
                .file_name("test.wav")
                .mime_str("audio/wav")
                .map_err(|e| e.to_string())?;
            let mut form = reqwest::multipart::Form::new()
                .text("model", cfg.model.to_string())
                .part("file", file_part);
            for &(key, value) in cfg.extra_fields {
                form = form.text(key.to_string(), value.to_string());
            }

            client
                .post(cfg.endpoint)
                .header("Authorization", format!("Bearer {}", api_key))
                .multipart(form)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub async fn test_stt_connection(
    api_key: String,
    provider: String,
    token_store: tauri::State<'_, SessionTokenStore>,
    client: tauri::State<'_, reqwest::Client>,
) -> Result<bool, String> {
    if provider.is_empty() {
        return Ok(false);
    }

    // Cloud provider: verify session token + Pro status via API
    if provider == "cloud" {
        let resp = send_stt_test_request(&provider, &api_key, &client, &token_store).await?;
        if !resp.status().is_success() {
            return Ok(false);
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        return Ok(body["plan"].as_str() == Some("pro"));
    }

    if api_key.is_empty() {
        return Ok(false);
    }

    let resp = send_stt_test_request(&provider, &api_key, &client, &token_store).await?;
    Ok(resp.status().is_success())
}

#[tauri::command]
pub async fn bench_stt_connection(
    api_key: String,
    provider: String,
    token_store: tauri::State<'_, SessionTokenStore>,
    client: tauri::State<'_, reqwest::Client>,
) -> Result<u32, String> {
    if provider.is_empty() {
        return Err("No provider specified".to_string());
    }

    if provider == "cloud" {
        let t0 = std::time::Instant::now();
        let resp = send_stt_test_request(&provider, &api_key, &client, &token_store).await?;
        let elapsed = t0.elapsed().as_millis() as u32;
        if !resp.status().is_success() {
            return Err("Request failed".to_string());
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if body["plan"].as_str() != Some("pro") {
            return Err("Pro plan required".to_string());
        }
        return Ok(elapsed);
    }

    if api_key.is_empty() {
        return Err("API key is empty".to_string());
    }

    let t0 = std::time::Instant::now();
    let resp = send_stt_test_request(&provider, &api_key, &client, &token_store).await?;
    let elapsed = t0.elapsed().as_millis() as u32;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_multipart_form_contains_model() {
        let cfg = stt::config::get_whisper_config("glm-asr").unwrap();
        assert_eq!(cfg.model, "glm-asr-2512");
    }

    #[test]
    fn test_whisper_multipart_form_contains_extra_fields() {
        let cfg = stt::config::get_whisper_config("glm-asr").unwrap();
        assert!(cfg.extra_fields.contains(&("stream", "false")));
    }

    #[test]
    fn test_build_whisper_test_wav() {
        let silent_pcm = vec![0u8; 3200];
        let wav = stt::whisper_compat::WhisperCompatProvider::build_wav(&silent_pcm, 16000);

        // WAV should be 44 header + 3200 data = 3244 bytes
        assert_eq!(wav.len(), 3244);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_openai_whisper_has_no_extra_fields() {
        let cfg = stt::config::get_whisper_config("openai-whisper").unwrap();
        assert_eq!(cfg.model, "whisper-1");
        assert!(cfg.extra_fields.is_empty());
    }

    #[test]
    fn test_unknown_provider_errors() {
        let result = stt::config::get_whisper_config("nonexistent");
        assert!(result.is_none());
    }
}
