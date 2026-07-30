//! OBS browser-source output.
//!
//! A tiny hand-rolled HTTP/1.1 server (std::net only) that serves the *same*
//! overlay frontend (`overlay.js` / `devices.js` / `style.css`) as the native
//! window, plus a Server-Sent-Events stream of the input events. A small
//! `window.__TAURI__` shim (`tauri-shim.js`) lets that unmodified frontend run
//! in a plain browser, so OBS can point a Browser source at
//! `http://localhost:<port>/overlay` and get exactly what the desktop overlay
//! shows — transparent and live.
//!
//! We write the socket directly (rather than via a crate) so SSE frames are
//! flushed the instant they are produced; buffered HTTP servers break SSE.
//!
//! Localhost-only; only the overlay assets and the current config are served.
//! Opt-in via `obsServerEnabled`.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::config::{Config, SharedConfig};

// Frontend assets, embedded so the server needs no filesystem access and stays
// consistent with the binary. Paths are relative to this source file.
const OBS_HTML: &str = include_str!("../../src/obs.html");
const SHIM_JS: &str = include_str!("../../src/tauri-shim.js");
const OVERLAY_JS: &str = include_str!("../../src/overlay.js");
const DEVICES_JS: &str = include_str!("../../src/devices.js");
const KEYBOARD_LAYOUT_JS: &str = include_str!("../../src/keyboard-layout.js");
const KEYBOARD_JS: &str = include_str!("../../src/keyboard.js");
const STYLE_CSS: &str = include_str!("../../src/style.css");

/// How often an idle SSE connection gets a heartbeat comment — keeps the
/// connection alive and lets us notice a client that has gone away.
const HEARTBEAT: Duration = Duration::from_secs(15);

/// Fan-out of overlay events to every connected browser (OBS) client. Each
/// client is an SSE connection draining an mpsc channel of pre-framed bytes.
#[derive(Default)]
pub struct ObsHub {
    clients: Mutex<Vec<mpsc::Sender<Vec<u8>>>>,
    started: AtomicBool,
}

impl ObsHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_clients(&self) -> bool {
        self.clients
            .lock()
            .map(|clients| !clients.is_empty())
            .unwrap_or(false)
    }

    /// Sends `{event, payload}` to every connected client as one SSE frame.
    /// `payload_json` must already be valid JSON. Dead clients are dropped.
    pub fn broadcast(&self, event: &str, payload_json: &str) {
        let frame =
            format!("data: {{\"event\":\"{event}\",\"payload\":{payload_json}}}\n\n").into_bytes();
        if let Ok(mut clients) = self.clients.lock() {
            clients.retain(|tx| tx.send(frame.clone()).is_ok());
        }
    }

    fn subscribe(&self) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut clients) = self.clients.lock() {
            clients.push(tx);
        }
        rx
    }
}

/// True when at least one browser client is connected — lets hot paths skip
/// serialization when nobody is listening.
pub fn has_clients(app: &AppHandle) -> bool {
    app.try_state::<Arc<ObsHub>>()
        .map(|hub| hub.has_clients())
        .unwrap_or(false)
}

/// Broadcasts an overlay event to browser clients, mirroring an `app.emit`.
pub fn broadcast(app: &AppHandle, event: &str, payload_json: &str) {
    if let Some(hub) = app.try_state::<Arc<ObsHub>>() {
        hub.broadcast(event, payload_json);
    }
}

/// Starts the server if the config enables it and it isn't already running.
/// Called at boot and after every settings save, so enabling the OBS source
/// takes effect immediately (no restart). Port changes still need a restart.
pub fn ensure_started(app: &AppHandle, config: &Config) {
    if !config.obs_server_enabled {
        return;
    }
    let (Some(hub), Some(shared)) = (
        app.try_state::<Arc<ObsHub>>(),
        app.try_state::<SharedConfig>(),
    ) else {
        return;
    };
    // Start at most once.
    if hub.started.swap(true, Ordering::SeqCst) {
        return;
    }
    spawn((*shared).clone(), (*hub).clone(), config.obs_server_port);
}

/// Binds `127.0.0.1:<port>` and serves connections, one thread per connection.
pub fn spawn(config: SharedConfig, hub: Arc<ObsHub>, port: u16) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("YAKC: OBS server could not bind port {port}: {err}");
                return;
            }
        };
        eprintln!("YAKC: OBS browser source ready at http://localhost:{port}/overlay");

        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let config = config.clone();
            let hub = hub.clone();
            // One thread per connection: SSE streams block for their lifetime.
            std::thread::spawn(move || handle(stream, &config, &hub));
        }
    });
}

fn handle(stream: TcpStream, config: &SharedConfig, hub: &Arc<ObsHub>) {
    // Read just the request line ("GET /path HTTP/1.1"); we don't need headers.
    let Ok(peek) = stream.try_clone() else { return };
    let mut request_line = String::new();
    if BufReader::new(peek).read_line(&mut request_line).is_err() {
        return;
    }
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();

    match path.as_str() {
        "/" | "/overlay" => respond(stream, "text/html; charset=utf-8", OBS_HTML),
        "/tauri-shim.js" => respond(stream, "text/javascript; charset=utf-8", SHIM_JS),
        "/overlay.js" => respond(stream, "text/javascript; charset=utf-8", OVERLAY_JS),
        "/devices.js" => respond(stream, "text/javascript; charset=utf-8", DEVICES_JS),
        "/keyboard-layout.js" => respond(stream, "text/javascript; charset=utf-8", KEYBOARD_LAYOUT_JS),
        "/keyboard.js" => respond(stream, "text/javascript; charset=utf-8", KEYBOARD_JS),
        "/style.css" => respond(stream, "text/css; charset=utf-8", STYLE_CSS),
        "/config" => {
            let json = config
                .read()
                .ok()
                .and_then(|cfg| serde_json::to_string(&*cfg).ok())
                .unwrap_or_else(|| "{}".to_string());
            respond(stream, "application/json; charset=utf-8", &json);
        }
        "/key_labels" => {
            let layout = config
                .read()
                .ok()
                .map(|cfg| cfg.keyboard_layout.clone())
                .filter(|layout| !layout.trim().is_empty());
            let labels = crate::input::key_labels(layout.as_deref());
            let json = serde_json::to_string(&labels).unwrap_or_else(|_| "{}".to_string());
            respond(stream, "application/json; charset=utf-8", &json);
        }
        "/events" => stream_events(stream, hub),
        _ => {
            let _ = write!(
                &stream,
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
        }
    }
}

/// Writes a complete (finite) response with a Content-Length, then closes.
fn respond(mut stream: TcpStream, content_type: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\r\n",
        len = body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

/// Holds the connection open and forwards broadcast frames as they arrive,
/// flushing each one immediately so events reach the browser with no delay.
fn stream_events(mut stream: TcpStream, hub: &Arc<ObsHub>) {
    let head = "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: keep-alive\r\n\r\n";
    if stream.write_all(head.as_bytes()).is_err() || stream.flush().is_err() {
        return;
    }
    // Open the stream immediately so EventSource fires `onopen`.
    if stream.write_all(b": connected\n\n").is_err() || stream.flush().is_err() {
        return;
    }

    let rx = hub.subscribe();
    loop {
        match rx.recv_timeout(HEARTBEAT) {
            Ok(frame) => {
                if stream.write_all(&frame).is_err() || stream.flush().is_err() {
                    break; // client gone; dropping rx prunes it from the hub
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if stream.write_all(b": ping\n\n").is_err() || stream.flush().is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
