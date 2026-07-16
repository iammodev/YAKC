//! Text-to-speech with identical behavior on every platform (SAPI on Windows,
//! AVSpeechSynthesizer on macOS, speech-dispatcher on Linux) via the `tts`
//! crate. The WebView speech API is not used because WebKitGTK lacks it.

pub struct Speaker(Option<tts::Tts>);

impl Speaker {
    pub fn available(&self) -> bool {
        self.0.is_some()
    }

    pub fn new() -> Self {
        match tts::Tts::default() {
            Ok(engine) => Self(Some(engine)),
            Err(err) => {
                eprintln!("YAKC: text-to-speech unavailable: {err}");
                Self(None)
            }
        }
    }

    /// Speaks `text`; `interrupt` cancels any ongoing speech first
    /// (textToSpeechCancelSpeechOnNewKey).
    pub fn speak(&mut self, text: &str, interrupt: bool) {
        if let Some(engine) = &mut self.0 {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }
            if let Err(err) = engine.speak(trimmed, interrupt) {
                eprintln!("YAKC: text-to-speech error: {err}");
            }
        }
    }
}
