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
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let value = value.strip_prefix('#').unwrap_or(value);
        if !value.is_ascii() {
            return None;
        }
        let value = match value.len() {
            3 => {
                let mut expanded = String::with_capacity(6);
                for byte in value.bytes() {
                    let digit = char::from(byte);
                    expanded.push(digit);
                    expanded.push(digit);
                }
                return Self::from_hex(&expanded);
            }
            6 => value,
            _ => return None,
        };
        let red = u8::from_str_radix(&value[0..2], 16).ok()?;
        let green = u8::from_str_radix(&value[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
        Some(Self::new(red, green, blue))
    }

    pub fn normalized(self) -> (f64, f64, f64) {
        (
            f64::from(self.red) / 255.0,
            f64::from(self.green) / 255.0,
            f64::from(self.blue) / 255.0,
        )
    }

    pub fn gradient(self) -> Gradient {
        Gradient {
            top: self,
            bottom: Self::new(
                self.red.saturating_mul(4) / 5,
                self.green.saturating_mul(4) / 5,
                self.blue.saturating_mul(4) / 5,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gradient {
    pub top: Rgb,
    pub bottom: Rgb,
}

impl Gradient {
    pub fn color_at(self, position: f64) -> Rgb {
        let position = position.clamp(0.0, 1.0);
        let channel = |top: u8, bottom: u8| {
            (f64::from(top) + (f64::from(bottom) - f64::from(top)) * position).round() as u8
        };
        Rgb::new(
            channel(self.top.red, self.bottom.red),
            channel(self.top.green, self.bottom.green),
            channel(self.top.blue, self.bottom.blue),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub hour_format: HourFormat,
    pub show_seconds: bool,
    pub color: Option<Rgb>,
    pub gradient: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hour_format: HourFormat::H24,
            show_seconds: true,
            color: None,
            gradient: false,
        }
    }
}
