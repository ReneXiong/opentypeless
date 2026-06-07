pub mod openai;
pub mod prompt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::llm::{AppType, ChunkCallback, LlmConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalRequest {
    /// WAV-encoded audio (PCM 16-bit mono 16kHz wrapped in WAV header)
    pub audio_wav: Vec<u8>,
    /// System prompt combining transcription + polish instructions
    pub system_prompt: String,
    /// App context type for tone/formatting
    pub app_type: AppType,
    /// Custom dictionary terms
    pub dictionary: Vec<String>,
    /// Whether translation is enabled
    pub translate_enabled: bool,
    /// Target language code for translation
    pub target_lang: String,
    /// Selected text in the target app (for "selected text mode")
    pub selected_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalResponse {
    pub text: String,
}

#[async_trait]
pub trait MultimodalProvider: Send + Sync {
    async fn process(
        &self,
        config: &LlmConfig,
        req: &MultimodalRequest,
        on_chunk: Option<&ChunkCallback>,
    ) -> Result<MultimodalResponse, AppError>;

    fn name(&self) -> &str;
}

/// Create a multimodal provider by name.
/// Uses the same routing as the LLM provider factory:
/// - "cloud" → CloudMultimodalProvider (future)
/// - everything else → OpenAiMultimodalProvider (OpenAI-compatible audio input)
pub fn create_provider(
    provider_name: &str,
    client: Option<reqwest::Client>,
) -> Box<dyn MultimodalProvider> {
    match (provider_name, client) {
        // Future: ("cloud", _) => Box::new(cloud::CloudMultimodalProvider::new()),
        (_, Some(c)) => Box::new(openai::OpenAiMultimodalProvider::with_client(c)),
        (_, None) => Box::new(openai::OpenAiMultimodalProvider::new()),
    }
}

/// Build a WAV file from raw PCM 16-bit mono audio.
/// Reuses the same logic as Deepgram/Whisper providers.
pub fn build_wav(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
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
