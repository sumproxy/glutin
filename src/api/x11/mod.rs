#![cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "openbsd"))]

pub use self::monitor::{MonitorId, get_available_monitors, get_primary_monitor};
pub use self::window::{Window, XWindow, PollEventsIterator, WaitEventsIterator, Context, WindowProxy};

pub use winit::api::x11::XError;
pub use winit::api::x11::XNotSupported;
pub use winit::api::x11::XConnection;

pub mod ffi;

mod events;
mod input;
mod monitor;
mod window;
