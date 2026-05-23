use std::fmt;

use winit::event_loop::EventLoopProxy;

use crate::app::UserEvent;

#[derive(Debug)]
pub enum ClipboardError {
    Access(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::Access(msg) => write!(f, "clipboard error: {}", msg),
        }
    }
}

/// OS-side clipboard. Owned by the main winit thread because macOS pasteboard
/// APIs are tied to the main thread.
pub struct SystemClipboard {
    inner: arboard::Clipboard,
}

impl SystemClipboard {
    pub fn new() -> Result<Self, ClipboardError> {
        arboard::Clipboard::new()
            .map(|c| Self { inner: c })
            .map_err(|e| ClipboardError::Access(e.to_string()))
    }

    pub fn read_text(&mut self) -> Result<Option<String>, ClipboardError> {
        match self.inner.get_text() {
            Ok(text) => {
                if text.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(text))
                }
            }
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(e) => Err(ClipboardError::Access(e.to_string())),
        }
    }

    pub fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.inner
            .set_text(text)
            .map_err(|e| ClipboardError::Access(e.to_string()))
    }
}

/// JS-thread side of the clipboard. Forwards reads/writes to the main thread
/// over `UserEvent` and blocks on a flume reply. Cheap because clipboard ops
/// are an OS round-trip anyway.
pub struct ClipboardBridge<'a> {
    proxy: &'a EventLoopProxy<UserEvent>,
}

impl<'a> ClipboardBridge<'a> {
    pub fn new(proxy: &'a EventLoopProxy<UserEvent>) -> Self {
        Self { proxy }
    }

    pub fn read_text(&self) -> Result<Option<String>, ClipboardError> {
        let (tx, rx) = flume::bounded(1);
        self.proxy
            .send_event(UserEvent::ClipboardRead { reply: tx })
            .map_err(|_| ClipboardError::Access("event loop closed".into()))?;
        rx.recv()
            .map_err(|_| ClipboardError::Access("clipboard reply dropped".into()))
    }

    pub fn write_text(&self, text: &str) -> Result<(), ClipboardError> {
        let (tx, rx) = flume::bounded(1);
        self.proxy
            .send_event(UserEvent::ClipboardWrite {
                text: text.to_string(),
                reply: tx,
            })
            .map_err(|_| ClipboardError::Access("event loop closed".into()))?;
        let ok = rx
            .recv()
            .map_err(|_| ClipboardError::Access("clipboard reply dropped".into()))?;
        if ok {
            Ok(())
        } else {
            Err(ClipboardError::Access("write failed".into()))
        }
    }
}
