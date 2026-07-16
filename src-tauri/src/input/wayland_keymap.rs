//! Fetches the compositor's active xkb keymap over the Wayland protocol
//! (wl_seat → wl_keyboard → keymap event). This is the authoritative source:
//! it reflects exactly what the user configured in their desktop settings,
//! including variants and options, on every compositor.

use std::os::fd::AsRawFd;

use wayland_client::protocol::{wl_keyboard, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};

#[derive(Default)]
struct Fetcher {
    keymap: Option<String>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    failed: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Fetcher {
    fn event(
        _state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "wl_seat" {
                registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(7), qh, ());
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for Fetcher {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: wayland_client::WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            } else if !caps.contains(wl_seat::Capability::Keyboard) {
                state.failed = true;
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for Fetcher {
    fn event(
        state: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Keymap { format, fd, size } = event {
            match format {
                wayland_client::WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) => {
                    state.keymap = read_keymap_fd(fd.as_raw_fd(), size as usize);
                    if state.keymap.is_none() {
                        state.failed = true;
                    }
                }
                _ => state.failed = true,
            }
        }
    }
}

/// The keymap arrives as a (usually memfd/shm) file descriptor; mmap it
/// read-only and copy out the NUL-terminated keymap text.
fn read_keymap_fd(fd: i32, size: usize) -> Option<String> {
    if size == 0 {
        return None;
    }
    unsafe {
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            fd,
            0,
        );
        if ptr == libc::MAP_FAILED {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr as *const u8, size);
        let text = std::str::from_utf8(bytes.split(|&b| b == 0).next().unwrap_or(bytes))
            .ok()
            .map(|s| s.to_string());
        libc::munmap(ptr, size);
        text
    }
}

/// Connects to the Wayland compositor and returns the active keymap as xkb
/// keymap text, or None when unavailable (no compositor, no keyboard, …).
pub fn fetch() -> Option<String> {
    let conn = Connection::connect_to_env().ok()?;
    let display = conn.display();
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    let _registry = display.get_registry(&qh, ());

    let mut fetcher = Fetcher::default();
    // registry globals → seat capabilities → keymap: three round trips,
    // plus headroom; bail out early on success or explicit failure.
    for _ in 0..6 {
        queue.blocking_dispatch(&mut fetcher).ok()?;
        if fetcher.keymap.is_some() || fetcher.failed {
            break;
        }
    }
    fetcher.keymap
}
