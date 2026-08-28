use std::ffi::{CStr, c_char};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{HourFormat, Options};

const TM_STORAGE_WORDS: usize = 16;

/// Opaque, correctly aligned storage for the platform's `struct tm`.
///
/// The supported 64-bit Darwin and glibc targets use a `struct tm` smaller than
/// this 128-byte allocation. It never crosses the FFI boundary by value.
#[repr(C)]
struct TmStorage {
    words: [usize; TM_STORAGE_WORDS],
}

unsafe extern "C" {
    fn localtime_r(timestamp: *const i64, result: *mut TmStorage) -> *mut TmStorage;
    fn strftime(
        output: *mut c_char,
        max_size: usize,
        format: *const c_char,
        time: *const TmStorage,
    ) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockSnapshot {
    pub face_text: String,
    pub compact_text: String,
    pub date_text: String,
    pub meridiem: Option<&'static str>,
}

impl ClockSnapshot {
    pub fn now(options: Options) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64;
        Self::from_timestamp(timestamp, options)
    }

    pub fn from_timestamp(timestamp: i64, options: Options) -> Self {
        let mut tm = TmStorage {
            words: [0; TM_STORAGE_WORDS],
        };
        // SAFETY: `timestamp` and `tm` are valid pointers for the duration of
        // this call. `TmStorage` is aligned and larger than `struct tm` on the
        // explicitly supported 64-bit Darwin and glibc targets.
        let local = unsafe { localtime_r(&timestamp, &mut tm) };
        if local.is_null() {
            return Self::utc_fallback(timestamp, options);
        }

        let time_format: &CStr = if options.hour_format == HourFormat::H12 {
            if options.show_seconds {
                c"%I:%M:%S"
            } else {
                c"%I:%M"
            }
        } else if options.show_seconds {
            c"%H:%M:%S"
        } else {
            c"%H:%M"
        };
        let mut time = [0_i8; 32];
        // SAFETY: Both buffers are valid and NUL-terminated where required;
        // `tm` was initialized by `localtime_r` above.
        let written = unsafe { strftime(time.as_mut_ptr(), time.len(), time_format.as_ptr(), &tm) };
        let mut date = [0_i8; 64];
        // SAFETY: Same invariants as the preceding `strftime` call.
        let date_written =
            unsafe { strftime(date.as_mut_ptr(), date.len(), c"%d/%m/%Y".as_ptr(), &tm) };
        let meridiem = if options.hour_format == HourFormat::H12 {
            let mut period = [0_i8; 4];
            // SAFETY: Same invariants as the preceding `strftime` call.
            let period_written =
                unsafe { strftime(period.as_mut_ptr(), period.len(), c"%p".as_ptr(), &tm) };
            (period_written > 0).then(|| if period[0] == b'P' as i8 { "PM" } else { "AM" })
        } else {
            None
        };
        let face_text = c_buffer(&time, written).unwrap_or_else(|| "--:--".to_owned());
        let compact_text = meridiem
            .map(|period| format!("{face_text} {period}"))
            .unwrap_or_else(|| face_text.clone());
        Self {
            face_text,
            compact_text,
            date_text: c_buffer(&date, date_written).unwrap_or_else(|| "Local time".to_owned()),
            meridiem,
        }
    }

    fn utc_fallback(timestamp: i64, options: Options) -> Self {
        let seconds_per_day = 86_400;
        let day_seconds = timestamp.rem_euclid(seconds_per_day);
        let hour_24 = day_seconds / 3_600;
        let minute = (day_seconds % 3_600) / 60;
        let second = day_seconds % 60;
        let (hour, meridiem) = match options.hour_format {
            HourFormat::H24 => (hour_24, None),
            HourFormat::H12 => {
                let period = if hour_24 >= 12 { "PM" } else { "AM" };
                (
                    if hour_24 % 12 == 0 { 12 } else { hour_24 % 12 },
                    Some(period),
                )
            }
        };
        let face_text = if options.show_seconds {
            format!("{hour:02}:{minute:02}:{second:02}")
        } else {
            format!("{hour:02}:{minute:02}")
        };
        let compact_text = meridiem
            .map(|period| format!("{face_text} {period}"))
            .unwrap_or_else(|| face_text.clone());
        Self {
            face_text,
            compact_text,
            date_text: "UTC (local time unavailable)".to_owned(),
            meridiem,
        }
    }
}

fn c_buffer(buffer: &[i8], length: usize) -> Option<String> {
    (length > 0 && length <= buffer.len()).then(|| {
        String::from_utf8_lossy(
            &buffer[..length]
                .iter()
                .map(|byte| *byte as u8)
                .collect::<Vec<_>>(),
        )
        .into_owned()
    })
}

pub fn next_second_delay() -> Duration {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    Duration::from_secs(1).saturating_sub(Duration::from_nanos(u64::from(elapsed.subsec_nanos())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_fallback_formats_seconds() {
        let snapshot = ClockSnapshot::utc_fallback(
            3_661,
            Options {
                hour_format: HourFormat::H24,
                show_seconds: true,
            },
        );
        assert_eq!(snapshot.face_text, "01:01:01");
    }

    #[test]
    fn next_delay_is_one_second_or_less() {
        assert!(next_second_delay() <= Duration::from_secs(1));
    }
}
