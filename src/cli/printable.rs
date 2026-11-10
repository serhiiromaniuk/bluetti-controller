//! Terminal-safe text for the names stations advertise.

/// `text` with every control character (ESC, CR, C1 …) and every Unicode
/// bidi control (U+202E …) replaced by U+FFFD, so an advertised name can
/// neither drive the terminal nor reorder the line (SECURITY.md "Bounded
/// queries and clean output", amendment A5).
pub(crate) fn printable(text: &str) -> String {
    text.chars()
        .map(|c| {
            let bidi = matches!(
                c,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
            );
            if c.is_control() || bidi { '\u{fffd}' } else { c }
        })
        .collect()
}
