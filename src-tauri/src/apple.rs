//! The model Apple ships with the system, reached through the framework meant
//! for applications. Nothing to download, nothing to start, no terms for the
//! user to accept by hand.
//!
//! Only the answer comes from here. Apple exposes no embeddings, so the memory
//! stays on the engine compiled into this binary.
use crate::db::Res;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Why the model cannot answer. Matches the Swift side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Ready,
    DeviceNotEligible,
    IntelligenceOff,
    ModelNotReady,
    SystemTooOld,
    Unknown,
}

impl Availability {
    fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Ready,
            1 => Self::DeviceNotEligible,
            2 => Self::IntelligenceOff,
            3 => Self::ModelNotReady,
            4 => Self::SystemTooOld,
            _ => Self::Unknown,
        }
    }
    /// English is the source text the interface translates.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Ready => "",
            Self::DeviceNotEligible => "This Mac cannot run Apple's on-device model.",
            Self::IntelligenceOff => {
                "Apple Intelligence is turned off. Turn it on in System Settings, then try again."
            }
            Self::ModelNotReady => {
                "Apple is still downloading its model. Try again in a few minutes."
            }
            Self::SystemTooOld => "Apple's on-device model needs macOS 26 or newer.",
            Self::Unknown => "Apple's on-device model is unavailable.",
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::DeviceNotEligible => "device",
            Self::IntelligenceOff => "off",
            Self::ModelNotReady => "downloading",
            Self::SystemTooOld => "system",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(apple_model)]
mod ffi {
    use std::os::raw::{c_char, c_int, c_void};
    unsafe extern "C" {
        pub fn langolier_apple_availability() -> c_int;
        pub fn langolier_apple_free(p: *mut c_char);
        pub fn langolier_apple_generate(
            instructions: *const c_char,
            prompt: *const c_char,
            temperature: f64,
            max_tokens: c_int,
            on_delta: extern "C" fn(*const c_char, *mut c_void) -> c_int,
            context: *mut c_void,
            error: *mut *mut c_char,
        ) -> c_int;
    }
}

#[cfg(not(apple_model))]
pub fn availability() -> Availability {
    Availability::SystemTooOld
}

#[cfg(apple_model)]
pub fn availability() -> Availability {
    Availability::from_code(unsafe { ffi::langolier_apple_availability() })
}

pub struct Answer {
    pub text: String,
    pub first_token_ms: Option<u64>,
}

#[cfg(not(apple_model))]
pub fn generate(
    _instructions: &str,
    _prompt: &str,
    _temperature: f64,
    _max_tokens: u32,
    _cancel: Arc<AtomicBool>,
    _on_token: &dyn Fn(&str),
) -> Res<Answer> {
    Err(Availability::SystemTooOld.reason().into())
}

/// What the callback carries across the bridge: where to put the text, and
/// whether the conversation is still wanted.
#[cfg(apple_model)]
struct Sink<'a> {
    text: String,
    started: std::time::Instant,
    first: Option<u64>,
    cancel: Arc<AtomicBool>,
    on_token: &'a dyn Fn(&str),
}

#[cfg(apple_model)]
extern "C" fn feed(
    piece: *const std::os::raw::c_char,
    context: *mut std::os::raw::c_void,
) -> std::os::raw::c_int {
    if piece.is_null() || context.is_null() {
        return 0;
    }
    // Safety: the pointer is the Sink handed to langolier_apple_generate, which
    // outlives the call, and Swift never calls back after it returns.
    let sink = unsafe { &mut *(context as *mut Sink) };
    if sink.cancel.load(Ordering::SeqCst) {
        return 0;
    }
    let Ok(text) = (unsafe { std::ffi::CStr::from_ptr(piece) }).to_str() else {
        return 0;
    };
    sink.first
        .get_or_insert(sink.started.elapsed().as_millis() as u64);
    sink.text.push_str(text);
    (sink.on_token)(text);
    1
}

#[cfg(apple_model)]
pub fn generate(
    instructions: &str,
    prompt: &str,
    temperature: f64,
    max_tokens: u32,
    cancel: Arc<AtomicBool>,
    on_token: &dyn Fn(&str),
) -> Res<Answer> {
    let ready = availability();
    if ready != Availability::Ready {
        return Err(ready.reason().into());
    }
    let brief = std::ffi::CString::new(instructions).map_err(|_| "instructions")?;
    let ask = std::ffi::CString::new(prompt).map_err(|_| "prompt")?;
    let mut sink = Sink {
        text: String::new(),
        started: std::time::Instant::now(),
        first: None,
        cancel: cancel.clone(),
        on_token,
    };
    let mut error: *mut std::os::raw::c_char = std::ptr::null_mut();
    let code = unsafe {
        ffi::langolier_apple_generate(
            brief.as_ptr(),
            ask.as_ptr(),
            temperature,
            max_tokens.min(i32::MAX as u32) as i32,
            feed,
            &mut sink as *mut Sink as *mut std::os::raw::c_void,
            &mut error,
        )
    };
    if !error.is_null() {
        let message = unsafe { std::ffi::CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::langolier_apple_free(error) };
        return Err(message);
    }
    if code != 0 {
        return Err("Apple's on-device model is unavailable.".into());
    }
    if cancel.load(Ordering::SeqCst) {
        return Err("Generation cancelled".into());
    }
    Ok(Answer {
        text: sink.text,
        first_token_ms: sink.first,
    })
}

/// Runs the bridge on its own thread and hands the pieces back as they
/// arrive, so the caller's closure never crosses a thread boundary. Same
/// shape as the engine compiled into this binary, for one calling style.
pub async fn answer(
    instructions: String,
    prompt: String,
    temperature: f64,
    max_tokens: u32,
    cancel: Arc<AtomicBool>,
    on_token: impl Fn(&str),
) -> Res<Answer> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let out = {
            let send = |piece: &str| {
                let _ = tx.send(piece.to_string());
            };
            generate(
                &instructions,
                &prompt,
                temperature,
                max_tokens,
                cancel,
                &send,
            )
        };
        drop(tx);
        let _ = done_tx.send(out);
    });
    while let Some(piece) = rx.recv().await {
        on_token(&piece);
    }
    done_rx.await.map_err(|_| "The model did not answer")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_reason_says_something_actionable() {
        for a in [
            Availability::DeviceNotEligible,
            Availability::IntelligenceOff,
            Availability::ModelNotReady,
            Availability::SystemTooOld,
            Availability::Unknown,
        ] {
            assert!(!a.reason().is_empty(), "{a:?} has no reason");
            assert!(!a.as_str().is_empty());
        }
        assert!(Availability::Ready.reason().is_empty());
    }
    /// Crosses the bridge for real.
    /// cargo test apple::tests::answers -- --ignored --nocapture
    #[test]
    #[ignore = "Needs a Mac whose on-device model is ready"]
    fn answers() {
        let ready = availability();
        println!("availability: {ready:?}");
        if ready != Availability::Ready {
            println!("skipped: {}", ready.reason());
            return;
        }
        let pieces = std::sync::Mutex::new(0usize);
        let start = std::time::Instant::now();
        let answer = generate(
            "Reponds en francais, en une phrase, a partir des passages fournis.",
            "[1] RRF fusionne plusieurs classements en un seul.\n\nQuestion: que fait RRF ?",
            0.2,
            200,
            Arc::new(AtomicBool::new(false)),
            &|_| *pieces.lock().unwrap() += 1,
        )
        .unwrap();
        println!(
            "{} pieces, premier token {:?} ms, total {} ms\n{}",
            pieces.lock().unwrap(),
            answer.first_token_ms,
            start.elapsed().as_millis(),
            answer.text
        );
        assert!(!answer.text.trim().is_empty());
    }

    #[test]
    fn codes_match_the_swift_side() {
        assert_eq!(Availability::from_code(0), Availability::Ready);
        assert_eq!(Availability::from_code(2), Availability::IntelligenceOff);
        assert_eq!(Availability::from_code(4), Availability::SystemTooOld);
        assert_eq!(Availability::from_code(99), Availability::Unknown);
    }
}
