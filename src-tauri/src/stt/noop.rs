use async_trait::async_trait;

use crate::error::AppError;

use super::{SttConfig, SttProvider, TranscriptEvent};

/// A no-op STT provider that does nothing.
/// Used in multimodal mode where audio is sent directly to the LLM.
pub struct NoopSttProvider;

impl NoopSttProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SttProvider for NoopSttProvider {
    async fn connect(&mut self, _config: &SttConfig) -> Result<(), AppError> {
        tracing::debug!("NoopSttProvider: connect (multimodal mode, STT skipped)");
        Ok(())
    }

    async fn send_audio(&mut self, _chunk: &[u8]) -> Result<(), AppError> {
        // Silently discard audio - in multimodal mode, audio is collected separately
        Ok(())
    }

    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> {
        // Never return transcripts - the pipeline will not wait for this in multimodal mode
        Ok(None)
    }

    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        tracing::debug!("NoopSttProvider: disconnect");
        Ok(None)
    }

    fn name(&self) -> &str {
        "noop"
    }
}
