use async_trait::async_trait;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::error::AppError;

use super::{OutputMode, TextOutput};

/// Maximum characters per enigo.text() call to avoid input buffer overflow.
const TYPE_CHUNK_SIZE: usize = 200;
/// Delay between typing chunks.
const TYPE_CHUNK_DELAY_MS: u64 = 5;

/// Check if keyboard simulation is reliable on this platform.
/// Returns Ok(()) if fine, or Err with a reason string for the caller.
pub fn check_keyboard_available() -> std::result::Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        if session == "wayland" {
            return Err("wayland_unsupported".to_string());
        }
        if (session == "x11" || session.is_empty())
            && std::process::Command::new("which")
                .arg("xdotool")
                .output()
                .map(|o| !o.status.success())
                .unwrap_or(true)
        {
            return Err("xdotool_missing".to_string());
        }
    }
    let _ = (); // suppress unused warning on non-Linux
    Ok(())
}

pub struct KeyboardOutput {
    app_handle: tauri::AppHandle,
}

impl Default for KeyboardOutput {
    fn default() -> Self {
        panic!("KeyboardOutput::default() must not be used — use KeyboardOutput::new(app_handle) instead");
    }
}

impl KeyboardOutput {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl TextOutput for KeyboardOutput {
    async fn type_text(&self, text: &str) -> Result<(), AppError> {
        let text = text.to_string();
        let app_handle = self.app_handle.clone();
        // macOS 26.5 requires TSM APIs (used by Enigo::new()) to run on the
        // main dispatch queue. Enigo is !Send, so the entire typing operation
        // must happen on the main thread.
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = app_handle.run_on_main_thread(move || {
            let result = (|| -> Result<(), AppError> {
                let mut enigo = Enigo::new(&Settings::default())
                    .map_err(|e| AppError::Output(format!("Failed to create Enigo: {:?}", e)))?;

                let lines: Vec<&str> = text.split('\n').collect();
                for (i, line) in lines.iter().enumerate() {
                    if !line.is_empty() {
                        for chunk in line.chars().collect::<Vec<_>>().chunks(TYPE_CHUNK_SIZE) {
                            let s: String = chunk.iter().collect();
                            enigo.text(&s).map_err(|e| {
                                AppError::Output(format!("Failed to type text: {:?}", e))
                            })?;
                            std::thread::sleep(std::time::Duration::from_millis(
                                TYPE_CHUNK_DELAY_MS,
                            ));
                        }
                    }
                    if i < lines.len() - 1 {
                        enigo
                            .key(Key::Shift, Direction::Press)
                            .map_err(|e| AppError::Output(format!("Key error: {:?}", e)))?;
                        enigo
                            .key(Key::Return, Direction::Click)
                            .map_err(|e| AppError::Output(format!("Key error: {:?}", e)))?;
                        enigo
                            .key(Key::Shift, Direction::Release)
                            .map_err(|e| AppError::Output(format!("Key error: {:?}", e)))?;
                    }
                }

                Ok(())
            })();
            let _ = tx.send(result);
        });
        rx.recv()
            .map_err(|e| AppError::Output(format!("Main thread task dropped: {}", e)))?
    }

    fn mode(&self) -> OutputMode {
        OutputMode::Keyboard
    }
}
