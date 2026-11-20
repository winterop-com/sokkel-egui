//! What the web examples lean on the browser for, so each egui example gives the same answers as
//! its web original: how a number is rounded, read and written (`number`), how a text is read,
//! measured in UTF-16 units and cut (`text`), how `en-US` collates text (`collate`), the calendar
//! and its moments in UTC as `Date` counts and `Intl` writes them (`date`), JSON as `JSON.parse`
//! and `JSON.stringify` read and write it (`json`), and base64, query strings and URI components
//! (`uri`).
//!
//! These are stand-ins for the browser that the examples and their web tests hold to the web's
//! answers bit for bit, not parts of the design system: what the core's own parts write (a locale
//! number, a compact one, `toFixed`) is the core's, `sokkel_egui::numbers`, and used from there.
//! Everything is at the top of the module, as `js::to_fixed`, `js::locale_compare`, ...

mod collate;
mod date;
mod json;
mod number;
mod text;
mod uri;

pub use collate::*;
pub use date::*;
pub use json::*;
pub use number::*;
pub use text::*;
pub use uri::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_the_web_writes_them() {
        assert_eq!(number_to_string(0.5), "0.5");
        assert_eq!(number_to_string(15.5), "15.5");
        assert_eq!(number_to_string(-2.0), "-2");
        assert_eq!(number_to_string(160.0), "160");
        assert_eq!(number_to_string(0.1), "0.1");
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(123_456_789_012_345_680_000.0), "123456789012345680000");
        assert_eq!(number_to_string(1.5e-7), "1.5e-7");
        assert_eq!(number_to_string(0.000_001), "0.000001");
        assert_eq!(number_to_string(-2.5), "-2.5");
        assert_eq!(to_fixed(1.25, 1), "1.3");
        assert_eq!(to_fixed(1.35, 1), "1.4");
        assert_eq!(to_fixed(0.05, 1), "0.1");
        assert_eq!(to_fixed(-0.04, 1), "-0.0");
        assert_eq!(to_fixed(59.96, 1), "60.0");
        assert_eq!(to_fixed(1e21, 2), "1e+21");
        assert_eq!(locale_number(1234.0), "1,234");
        assert_eq!(locale_number(1_234_567.891_5), "1,234,567.892");
        assert_eq!(locale_number(0.5), "0.5");
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(-0.5), 0.0);
        assert_eq!(round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(base36(1_794_992_400_000), "mwlw4o00");
        assert_eq!(base36(1_794_992_401_000), "mwlw4ors");
        assert_eq!(base36(-35), "-z");
        assert_eq!(to_uint32(-1.0), 4_294_967_295);
    }

    #[test]
    fn texts_are_read_as_number_reads_them() {
        assert_eq!(number_of_text(" 12 "), 12.0);
        assert_eq!(number_of_text(" +250 "), 250.0);
        assert_eq!(number_of_text("0x1F"), 31.0);
        assert_eq!(number_of_text("0x10"), 16.0);
        assert_eq!(number_of_text("1."), 1.0);
        assert_eq!(number_of_text(".5"), 0.5);
        assert_eq!(number_of_text(""), 0.0);
        assert!(number_of_text("1.2.3").is_nan());
        assert!(number_of_text("inf").is_nan());
    }

    #[test]
    fn moments_and_texts_are_written_as_the_web_writes_them() {
        assert_eq!(iso_instant(utc(2026, 9, 20, 0, 0, 0)), "2026-10-20T00:00:00.000Z");
        assert_eq!(replace_all("a-b-c", "-", "[$&$$]"), "a[-$]b[-$]c");
        assert_eq!(replace_all("ab", "", "-"), "-a-b-");
        assert_eq!(slice("abcdef", 1.0, Some(-1.0)), "bcde");
    }
}
