//! Cross-platform gamepad/controller capture via gilrs (XInput on Windows,
//! IOKit/GameController on macOS, evdev on Linux — works on X11 and Wayland).
//!
//! Like the mouse-button path, this always runs and feeds the shared RawInput
//! channel; the consumer thread decides whether to show anything based on the
//! `show_gamepad` config flag.

use std::sync::mpsc::Sender;
use std::time::Duration;

use gilrs::{Axis, Button, Gilrs};

use super::RawInput;

pub fn spawn_listener(tx: Sender<RawInput>) {
    std::thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(gilrs) => gilrs,
            Err(err) => {
                eprintln!("YAKC: gamepad support unavailable: {err}");
                return;
            }
        };

        // A controller may already be connected at startup.
        if gilrs.gamepads().next().is_some() {
            let _ = tx.send(RawInput::GamepadConnection { connected: true });
        }

        loop {
            while let Some(event) = gilrs.next_event() {
                match event.event {
                    gilrs::EventType::Connected => {
                        let _ = tx.send(RawInput::GamepadConnection { connected: true });
                    }
                    gilrs::EventType::Disconnected => {
                        let _ = tx.send(RawInput::GamepadConnection { connected: false });
                    }
                    gilrs::EventType::ButtonPressed(button, _) => {
                        if let Some(id) = button_id(button) {
                            let _ = tx.send(RawInput::GamepadButton { id, pressed: true });
                        }
                    }
                    gilrs::EventType::ButtonReleased(button, _) => {
                        if let Some(id) = button_id(button) {
                            let _ = tx.send(RawInput::GamepadButton { id, pressed: false });
                        }
                    }
                    gilrs::EventType::ButtonChanged(button, value, _) => {
                        // Analog triggers report their travel (0.0..1.0) here.
                        if let Some(axis) = trigger_axis(button) {
                            let _ = tx.send(RawInput::GamepadAxis {
                                axis,
                                value: value as f64,
                            });
                        }
                    }
                    gilrs::EventType::AxisChanged(axis, value, _) => {
                        if let Some(axis) = stick_axis(axis) {
                            let _ = tx.send(RawInput::GamepadAxis {
                                axis,
                                value: value as f64,
                            });
                        }
                    }
                    _ => {}
                }
            }
            // gilrs is poll-based; a short sleep keeps this thread near-idle
            // while staying responsive (~250 Hz).
            std::thread::sleep(Duration::from_millis(4));
        }
    });
}

/// Maps gilrs buttons to the shared ids used by keymap.rs / known_keys().
fn button_id(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::South => "gp_a",
        Button::East => "gp_b",
        Button::West => "gp_x",
        Button::North => "gp_y",
        Button::LeftTrigger => "gp_lb",
        Button::RightTrigger => "gp_rb",
        Button::LeftTrigger2 => "gp_lt",
        Button::RightTrigger2 => "gp_rt",
        Button::Select => "gp_back",
        Button::Start => "gp_start",
        Button::Mode => "gp_guide",
        Button::LeftThumb => "gp_ls",
        Button::RightThumb => "gp_rs",
        Button::DPadUp => "dpad_up",
        Button::DPadDown => "dpad_down",
        Button::DPadLeft => "dpad_left",
        Button::DPadRight => "dpad_right",
        _ => return None,
    })
}

/// Analog trigger travel is surfaced as an axis so the widget can draw a bar.
fn trigger_axis(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::LeftTrigger2 => "lt",
        Button::RightTrigger2 => "rt",
        _ => return None,
    })
}

fn stick_axis(axis: Axis) -> Option<&'static str> {
    Some(match axis {
        Axis::LeftStickX => "ls_x",
        Axis::LeftStickY => "ls_y",
        Axis::RightStickX => "rs_x",
        Axis::RightStickY => "rs_y",
        Axis::LeftZ => "lt",
        Axis::RightZ => "rt",
        _ => return None,
    })
}
