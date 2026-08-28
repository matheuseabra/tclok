//! Raster rendering of the user-installed Fira Code Bold face for Ghostty.
//!
//! Plain ANSI terminals cannot choose a per-application font or scale text.
//! On macOS Ghostty, tclok uses CoreGraphics to draw the installed face and
//! transmits the resulting pixels through the Kitty graphics protocol.

use crate::clock::ClockSnapshot;
use crate::layout::TerminalSize;
use crate::{Gradient, Rgb};

#[cfg(any(target_os = "macos", test))]
const IMAGE_ID: u32 = 1_624_011;
#[cfg(any(target_os = "macos", test))]
const ESC: &str = "\x1b";

pub fn render(
    size: TerminalSize,
    pixels: Option<(u16, u16)>,
    foreground: Option<Rgb>,
    gradient: Option<Gradient>,
    clock: &ClockSnapshot,
) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        macos::render(size, pixels, foreground, gradient, clock)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (size, pixels, foreground, gradient, clock);
        None
    }
}

#[cfg(any(target_os = "macos", test))]
fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let packed = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        encoded.push(TABLE[((packed >> 18) & 0x3f) as usize] as char);
        encoded.push(TABLE[((packed >> 12) & 0x3f) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[((packed >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(packed & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

#[cfg(any(target_os = "macos", test))]
fn kitty_frame(
    row: u16,
    column: u16,
    columns: u16,
    rows: u16,
    width: usize,
    height: usize,
    rgba: &[u8],
) -> String {
    let encoded = encode_base64(rgba);
    let chunks = encoded.as_bytes().chunks(4096).collect::<Vec<_>>();
    let mut frame = format!(
        "{ESC}[?2026h{ESC}_Ga=d,d=i,i={IMAGE_ID},q=2;{ESC}\\{ESC}[2J{ESC}[H{ESC}[{row};{column}H"
    );
    for (index, chunk) in chunks.iter().enumerate() {
        let more = usize::from(index + 1 < chunks.len());
        if index == 0 {
            frame.push_str(&format!(
                "{ESC}_Ga=T,f=32,s={width},v={height},i={IMAGE_ID},c={columns},r={rows},C=1,q=2,m={more};"
            ));
        } else {
            frame.push_str(&format!("{ESC}_Gm={more};"));
        }
        // Base64 uses ASCII only, so this conversion cannot fail.
        frame.push_str(std::str::from_utf8(chunk).expect("base64 output is ASCII"));
        frame.push_str("\x1b\\");
    }
    frame.push_str("\x1b[?2026l");
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_values() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
    }

    #[test]
    fn kitty_frame_uses_raw_rgba_and_does_not_draw_text_cells() {
        let frame = kitty_frame(2, 3, 10, 4, 2, 1, &[255; 8]);
        assert!(frame.starts_with("\x1b[?2026h"));
        assert!(frame.ends_with("\x1b[?2026l"));
        assert!(frame.contains("a=T,f=32,s=2,v=1"));
        assert!(frame.contains("c=10,r=4,C=1"));
        assert!(!frame.contains('█'));
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{c_char, c_int, c_void};

    use super::kitty_frame;
    use crate::{Gradient, Rgb, clock::ClockSnapshot, layout::TerminalSize};

    type CGFloat = f64;
    type CFTypeRef = *const c_void;
    type CGContextRef = *mut c_void;
    type CGFontRef = *const c_void;
    type CGGlyph = u16;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: CGFloat,
        y: CGFloat,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: CGFloat,
        height: CGFloat,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(
            allocator: *const c_void,
            string: *const c_char,
            encoding: u32,
        ) -> CFTypeRef;
        fn CFRelease(value: CFTypeRef);
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGColorSpaceCreateDeviceRGB() -> CFTypeRef;
        fn CGBitmapContextCreate(
            data: *mut c_void,
            width: usize,
            height: usize,
            bits_per_component: usize,
            bytes_per_row: usize,
            color_space: CFTypeRef,
            bitmap_info: u32,
        ) -> CGContextRef;
        fn CGContextRelease(context: CGContextRef);
        fn CGColorSpaceRelease(color_space: CFTypeRef);
        fn CGContextSetRGBFillColor(
            context: CGContextRef,
            red: CGFloat,
            green: CGFloat,
            blue: CGFloat,
            alpha: CGFloat,
        );
        fn CGContextSetTextDrawingMode(context: CGContextRef, mode: c_int);
        fn CGContextSetFont(context: CGContextRef, font: CGFontRef);
        fn CGContextSetFontSize(context: CGContextRef, size: CGFloat);
        fn CGContextShowGlyphsAtPositions(
            context: CGContextRef,
            glyphs: *const CGGlyph,
            positions: *const CGPoint,
            count: usize,
        );
        fn CGFontCreateWithFontName(name: CFTypeRef) -> CGFontRef;
        fn CGFontRelease(font: CGFontRef);
        fn CGFontGetUnitsPerEm(font: CGFontRef) -> u16;
        fn CGFontGetGlyphsForUnichars(
            font: CGFontRef,
            chars: *const u16,
            glyphs: *mut CGGlyph,
            count: usize,
        ) -> bool;
        fn CGFontGetGlyphAdvances(
            font: CGFontRef,
            glyphs: *const CGGlyph,
            count: usize,
            advances: *mut c_int,
        ) -> bool;
        fn CGFontGetGlyphBBoxes(
            font: CGFontRef,
            glyphs: *const CGGlyph,
            count: usize,
            boxes: *mut CGRect,
        ) -> bool;
    }

    const UTF8: u32 = 0x0800_0100;
    const RGBA_PREMULTIPLIED_LAST_BIG_ENDIAN: u32 = 0x4001;
    const KCG_TEXT_FILL: c_int = 0;
    const FULL_TIME_MIN_COLUMNS: u16 = 52;

    pub(super) fn render(
        size: TerminalSize,
        pixels: Option<(u16, u16)>,
        foreground: Option<Rgb>,
        gradient: Option<Gradient>,
        clock: &ClockSnapshot,
    ) -> Option<String> {
        if size.columns < 12 || size.rows < 4 {
            return None;
        }
        let columns = size.columns.saturating_sub(4);
        let rows = if size.rows >= 10 {
            7
        } else {
            size.rows.saturating_sub(1)
        };
        let (cell_width, cell_height) = pixels
            .filter(|(width, height)| *width > 0 && *height > 0)
            .map(|(width, height)| {
                (
                    usize::from(width) / usize::from(size.columns),
                    usize::from(height) / usize::from(size.rows),
                )
            })
            // Kitty scales this conservative 1:2 cell canvas to the actual
            // pane rectangle, so pixel metrics improve precision but are not
            // required to render a large real-font clock.
            .unwrap_or((10, 20));
        let width = usize::from(columns).checked_mul(cell_width)?;
        let height = usize::from(rows).checked_mul(cell_height)?;
        if width == 0 || height == 0 {
            return None;
        }
        let time = display_time(size, &clock.face_text);
        let rgba = rasterize(time, width, height, foreground, gradient)?;
        let row = (size.rows.saturating_sub(rows) / 2).saturating_add(1);
        let column = (size.columns.saturating_sub(columns) / 2).saturating_add(1);
        let mut frame = kitty_frame(row, column, columns, rows, width, height, &rgba);
        if size.rows >= 10 {
            let date_row = row.saturating_add(rows);
            if date_row <= size.rows {
                let date_width = clock.date_text.chars().count() as u16;
                let date_column = size.columns.saturating_sub(date_width.min(size.columns)) / 2 + 1;
                let date_color = gradient.map(|gradient| gradient.bottom).or(foreground);
                if let Some(color) = date_color {
                    frame.push_str(&format!(
                        "\x1b[38;2;{};{};{}m",
                        color.red, color.green, color.blue
                    ));
                }
                frame.push_str(&format!(
                    "\x1b[{date_row};{date_column}H{}",
                    clock.date_text
                ));
                if date_color.is_some() {
                    frame.push_str("\x1b[0m");
                }
            }
        }
        Some(frame)
    }

    fn display_time(size: TerminalSize, time: &str) -> &str {
        if size.columns < FULL_TIME_MIN_COLUMNS && time.matches(':').count() >= 2 {
            time.rsplit_once(':').map_or(time, |(head, _)| head)
        } else {
            time
        }
    }

    fn rasterize(
        text: &str,
        width: usize,
        height: usize,
        foreground: Option<Rgb>,
        gradient: Option<Gradient>,
    ) -> Option<Vec<u8>> {
        let c_name = b"FiraCode-Bold\0";
        // SAFETY: CoreFoundation copies this valid NUL-terminated UTF-8 name.
        let name =
            unsafe { CFStringCreateWithCString(std::ptr::null(), c_name.as_ptr().cast(), UTF8) };
        if name.is_null() {
            return None;
        }
        // SAFETY: `name` is a valid CoreFoundation string and is released below.
        let font = unsafe { CGFontCreateWithFontName(name) };
        // SAFETY: `name` is retained by us exactly once.
        unsafe { CFRelease(name) };
        if font.is_null() {
            return None;
        }
        let codepoints = text.encode_utf16().collect::<Vec<_>>();
        let mut glyphs = vec![0; codepoints.len()];
        // SAFETY: input/output buffers are valid for `codepoints.len()` elements.
        let _has_glyphs = unsafe {
            CGFontGetGlyphsForUnichars(font, codepoints.as_ptr(), glyphs.as_mut_ptr(), glyphs.len())
        };
        if glyphs.contains(&0) {
            // SAFETY: `font` is owned by this function.
            unsafe { CGFontRelease(font) };
            return None;
        }
        let mut advances = vec![0_i32; glyphs.len()];
        // SAFETY: input/output buffers are valid for `glyphs.len()` elements.
        let has_advances = unsafe {
            CGFontGetGlyphAdvances(font, glyphs.as_ptr(), glyphs.len(), advances.as_mut_ptr())
        };
        let units = unsafe { CGFontGetUnitsPerEm(font) };
        if !has_advances || units == 0 {
            // SAFETY: `font` is owned by this function.
            unsafe { CGFontRelease(font) };
            return None;
        }
        let mut boxes = vec![
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: 0.0,
                    height: 0.0,
                },
            };
            glyphs.len()
        ];
        // SAFETY: input/output buffers are valid for `glyphs.len()` elements.
        let has_boxes = unsafe {
            CGFontGetGlyphBBoxes(font, glyphs.as_ptr(), glyphs.len(), boxes.as_mut_ptr())
        };
        if !has_boxes {
            // SAFETY: `font` is owned by this function.
            unsafe { CGFontRelease(font) };
            return None;
        }
        let total_advance: CGFloat = advances.iter().map(|advance| CGFloat::from(*advance)).sum();
        if total_advance <= 0.0 {
            // SAFETY: `font` is owned by this function.
            unsafe { CGFontRelease(font) };
            return None;
        }
        let glyph_min_y = boxes
            .iter()
            .map(|bounds| bounds.origin.y)
            .fold(CGFloat::INFINITY, CGFloat::min);
        let glyph_max_y = boxes
            .iter()
            .map(|bounds| bounds.origin.y + bounds.size.height)
            .fold(CGFloat::NEG_INFINITY, CGFloat::max);
        let glyph_height = glyph_max_y - glyph_min_y;
        if glyph_height <= 0.0 {
            // SAFETY: `font` is owned by this function.
            unsafe { CGFontRelease(font) };
            return None;
        }
        let height_limit = height as CGFloat * 0.76 * CGFloat::from(units) / glyph_height;
        let width_limit = width as CGFloat * 0.92 * CGFloat::from(units) / total_advance;
        let font_size = height_limit.min(width_limit);
        let scale = font_size / CGFloat::from(units);
        let text_width = total_advance * scale;
        let mut x = (width as CGFloat - text_width) / 2.0;
        let baseline = (height as CGFloat - glyph_height * scale) / 2.0 - glyph_min_y * scale;
        let mut positions = Vec::with_capacity(glyphs.len());
        for advance in &advances {
            positions.push(CGPoint { x, y: baseline });
            x += CGFloat::from(*advance) * scale;
        }
        let mut rgba = vec![0; width.checked_mul(height)?.checked_mul(4)?];
        // SAFETY: CoreGraphics accesses `rgba` only while this context exists.
        let color_space = unsafe { CGColorSpaceCreateDeviceRGB() };
        // SAFETY: all dimensions and the RGBA buffer stride are valid.
        let context = unsafe {
            CGBitmapContextCreate(
                rgba.as_mut_ptr().cast(),
                width,
                height,
                8,
                width * 4,
                color_space,
                RGBA_PREMULTIPLIED_LAST_BIG_ENDIAN,
            )
        };
        // SAFETY: the context retained the colorspace if it was created.
        unsafe { CGColorSpaceRelease(color_space) };
        if context.is_null() {
            // SAFETY: `font` is owned by this function.
            unsafe { CGFontRelease(font) };
            return None;
        }
        // SAFETY: `context`, `font`, glyphs, and positions are valid for these calls.
        unsafe {
            CGContextSetTextDrawingMode(context, KCG_TEXT_FILL);
            let (red, green, blue) = if gradient.is_some() {
                (1.0, 1.0, 1.0)
            } else {
                foreground
                    .map(Rgb::normalized)
                    .unwrap_or((0.94, 0.94, 0.94))
            };
            CGContextSetRGBFillColor(context, red, green, blue, 1.0);
            CGContextSetFont(context, font);
            CGContextSetFontSize(context, font_size);
            CGContextShowGlyphsAtPositions(
                context,
                glyphs.as_ptr(),
                positions.as_ptr(),
                glyphs.len(),
            );
            CGContextRelease(context);
            CGFontRelease(font);
        }
        if let Some(gradient) = gradient {
            for row in 0..height {
                let color = gradient.color_at(row as f64 / height.saturating_sub(1).max(1) as f64);
                let (red, green, blue) = color.normalized();
                for column in 0..width {
                    let index = (row * width + column) * 4;
                    let alpha = f64::from(rgba[index + 3]) / 255.0;
                    rgba[index] = (red * alpha * 255.0).round() as u8;
                    rgba[index + 1] = (green * alpha * 255.0).round() as u8;
                    rgba[index + 2] = (blue * alpha * 255.0).round() as u8;
                }
            }
        }
        Some(rgba)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn installed_fira_code_bold_rasterizes_opaque_glyphs() {
            let Some(rgba) = rasterize("12:34", 640, 180, None, None) else {
                return;
            };
            let (pixels, remainder) = rgba.as_chunks::<4>();
            assert!(remainder.is_empty());
            assert!(pixels.iter().any(|pixel| pixel[3] > 0));
        }

        #[test]
        fn rasterized_face_uses_terminal_foreground_color() {
            let Some(rgba) = rasterize("12", 320, 180, Some(Rgb::new(51, 204, 102)), None) else {
                return;
            };
            let (pixels, remainder) = rgba.as_chunks::<4>();
            assert!(remainder.is_empty());
            assert!(pixels.iter().any(|pixel| {
                pixel[3] > 0
                    && f64::from(pixel[0]) / 255.0 < 0.35
                    && f64::from(pixel[1]) / 255.0 > 0.65
                    && f64::from(pixel[2]) / 255.0 < 0.55
            }));
        }

        #[test]
        fn missing_ioctl_pixel_metrics_still_emits_the_real_font_image() {
            let clock = ClockSnapshot {
                face_text: "12:34:56".into(),
                compact_text: "12:34:56".into(),
                date_text: "27/08/2026".into(),
                meridiem: None,
            };
            let Some(frame) = render(
                TerminalSize {
                    columns: 60,
                    rows: 10,
                },
                None,
                Some(Rgb::new(18, 52, 86)),
                None,
                &clock,
            ) else {
                return;
            };
            assert!(frame.contains("a=T,f=32"));
            assert!(frame.contains("27/08/2026"));
            assert!(frame.contains("\x1b[9;26H27/08/2026"));
            assert!(frame.contains("\x1b[38;2;18;52;86m"));
            assert!(!frame.contains('█'));
        }

        #[test]
        fn reduced_width_hides_seconds_before_leaving_the_large_face() {
            assert_eq!(
                display_time(
                    TerminalSize {
                        columns: 51,
                        rows: 7,
                    },
                    "12:34:56"
                ),
                "12:34"
            );
        }
    }
}
