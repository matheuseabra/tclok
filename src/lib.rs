//! Layout and rendering primitives for the `tclok` terminal clock.

#[cfg(not(all(
    target_pointer_width = "64",
    any(target_os = "linux", target_os = "macos")
)))]
compile_error!("tclok 0.1 supports 64-bit macOS and Linux only");

pub mod clock;
pub mod layout;
pub mod neue_machina;
pub mod render;
pub mod terminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HourFormat {
    H12,
    H24,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub hour_format: HourFormat,
    pub show_seconds: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hour_format: HourFormat::H24,
            show_seconds: true,
        }
    }
}
