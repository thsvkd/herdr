use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub(crate) fn display_width_u16(text: &str) -> u16 {
    display_width(text).min(u16::MAX as usize) as u16
}

pub(crate) fn truncate_end(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let prefix = take_prefix_width(text, max_width.saturating_sub(1));
    format!("{prefix}…")
}

pub(crate) fn middle_elide(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }

    let content_width = max_width.saturating_sub(1);
    let left_width = content_width / 2;
    let right_width = content_width.saturating_sub(left_width);
    let prefix = take_prefix_width(text, left_width);
    let suffix = take_suffix_width(text, right_width);
    format!("{prefix}…{suffix}")
}

/// Drops leading grapheme clusters from `text` until at least `skip_width`
/// display columns are gone, returning the columns actually dropped and the
/// remainder.
///
/// Clusters, not chars: a ZWJ or variation-selector sequence occupies one cell
/// that [`display_width`] measures as a unit, and the frame buffer lays it out
/// the same way. Summing per-char widths instead would both mis-report the
/// dropped columns and hand back a slice starting on a bare joiner.
///
/// The dropped width can still exceed `skip_width` when a cluster straddles the
/// boundary, so callers that align a caret must use the returned width rather
/// than `skip_width`.
pub(crate) fn skip_prefix_width(text: &str, skip_width: usize) -> (usize, &str) {
    let mut width = 0usize;
    for (idx, grapheme) in text.grapheme_indices(true) {
        if width >= skip_width {
            return (width, &text[idx..]);
        }
        width += display_width(grapheme);
    }
    (width, "")
}

fn take_prefix_width(text: &str, max_width: usize) -> String {
    let mut output = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        output.push(ch);
        width += ch_width;
    }
    output
}

fn take_suffix_width(text: &str, max_width: usize) -> String {
    let mut output = Vec::new();
    let mut width = 0usize;
    for ch in text.chars().rev() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_width {
            break;
        }
        output.push(ch);
        width += ch_width;
    }
    output.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_prefix_width_drops_whole_characters_only() {
        assert_eq!(skip_prefix_width("abcd", 0), (0, "abcd"));
        assert_eq!(skip_prefix_width("abcd", 2), (2, "cd"));
        assert_eq!(skip_prefix_width("abcd", 9), (4, ""));

        // A wide character straddling the boundary is dropped whole, so the
        // reported width overshoots the request.
        assert_eq!(skip_prefix_width("あa", 1), (2, "a"));
        assert_eq!(skip_prefix_width("あa", 2), (2, "a"));
    }

    #[test]
    fn skip_prefix_width_counts_grapheme_clusters_not_chars() {
        // A variation-selector sequence is one two-column cell. Summing the
        // chars would call it one column and cut between ❤ and the selector.
        let hearts = "\u{2764}\u{FE0F}".repeat(4);
        assert_eq!(display_width(&hearts), 8);
        let (skipped, rest) = skip_prefix_width(&hearts, 4);
        assert_eq!(skipped, 4);
        assert_eq!(rest, "\u{2764}\u{FE0F}\u{2764}\u{FE0F}");

        // A ZWJ family is six chars but still one two-column cell. A per-char
        // sum would report six columns dropped after the first cluster.
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let families = family.repeat(3);
        assert_eq!(display_width(&families), 6);
        let (skipped, rest) = skip_prefix_width(&families, 2);
        assert_eq!(skipped, 2);
        assert_eq!(rest, family.repeat(2));

        // Never hand back a slice that starts on a bare joiner or selector.
        for skip in 0..=display_width(&families) {
            let (_, rest) = skip_prefix_width(&families, skip);
            assert!(
                !rest.starts_with('\u{200D}') && !rest.starts_with('\u{FE0F}'),
                "skip {skip} split a cluster: {rest:?}"
            );
        }
    }

    #[test]
    fn truncate_end_uses_display_width() {
        let text = truncate_end("提交 herdr 的反馈", 16);

        assert_eq!(text, "提交 herdr 的反…");
        assert!(display_width(&text) <= 16);
    }

    #[test]
    fn middle_elide_uses_display_width() {
        let text = middle_elide("重构用户认证模块并迁移到统一登录服务", 12);

        assert!(text.contains('…'));
        assert!(display_width(&text) <= 12);
    }
}
