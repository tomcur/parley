// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::ops::Range;

use crate::shape::Whitespace;

/// Data for a single character of the source text.
///
/// This is a character in the Unicode scalar value sense, i.e., it corresponds to a single `char`
/// of the source text.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Character {
    /// The byte offset of this character in the source text.
    pub text_byte_start: u32,
    /// Style index for this character.
    pub style_index: u16,
    pub(crate) whitespace: Whitespace,
    pub(crate) flags: CharacterFlags,
}

impl Character {
    /// The byte range of this character in the source text.
    #[inline(always)]
    pub fn text_byte_range(&self) -> Range<usize> {
        self.text_byte_start as usize..self.text_byte_start as usize + self.len_utf8()
    }

    /// The whitespace class of this character.
    #[inline(always)]
    pub fn whitespace(self) -> Whitespace {
        self.whitespace
    }

    /// Whether this character is any whitespace.
    #[inline(always)]
    pub fn is_whitespace(self) -> bool {
        self.whitespace != Whitespace::None
    }

    /// Whether this character begins a grapheme cluster ([UAX #29 § 3][graphemes]).
    ///
    /// [graphemes]: https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
    #[inline(always)]
    pub fn is_grapheme_start(self) -> bool {
        self.flags.is_grapheme_start()
    }

    /// Whether there is a word boundary before this character ([UAX #29 § 4][words]).
    ///
    /// [words]: https://www.unicode.org/reports/tr29/#Word_Boundaries
    #[inline(always)]
    pub fn is_word_boundary(self) -> bool {
        self.flags.is_word_boundary()
    }

    /// Whether a line may be broken before this character ([UAX #14][]).
    ///
    /// This is true both at soft wrap opportunities and directly after a mandatory break
    /// character such as `\n`. Mandatory breaks themselves are identified by
    /// [`Whitespace::Newline`].
    ///
    /// [UAX #14]: https://www.unicode.org/reports/tr14/
    #[inline(always)]
    pub fn is_line_break_opportunity(self) -> bool {
        self.flags.is_line_break_opportunity()
    }

    /// Whether this character is an emoji.
    #[inline(always)]
    pub fn is_emoji(self) -> bool {
        self.flags.is_emoji()
    }

    /// Returns the number of bytes this character takes up in the (UTF-8) source text.
    ///
    /// That number of bytes is always between 1 and 4, inclusive.
    #[inline(always)]
    pub fn len_utf8(self) -> usize {
        self.flags.len_utf8()
    }
}

/// Per-[`Character`] properties packed into a byte.
///
/// The UTF-8 length sits at bit 0 so it can be extracted with a single mask, and the grapheme
/// start bit sits at the top so it can be extracted with a single shift (it is summed in hot
/// loops, see [`ShapedCluster::graphemes_overlapped`]). All other bits are only ever tested, for
/// which the position doesn't matter.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct CharacterFlags(u8);

impl CharacterFlags {
    /// Bits 0..2: the UTF-8 length of the character minus one.
    ///
    /// (Note the UTF-8 length of any character is between 1 and 4 inclusive.)
    const LEN_UTF8_MASK: u8 = 0b11;
    const LINE_BREAK_OPPORTUNITY: u8 = 1 << 2;
    const WORD_BOUNDARY: u8 = 1 << 3;
    const EMOJI: u8 = 1 << 4;
    // Bits 5 and 6 are spare.
    const GRAPHEME_START: u8 = 1 << 7;

    /// Flags for the character `ch`, with all boundary flags unset.
    #[inline(always)]
    pub(crate) fn new(ch: char) -> Self {
        // TODO: Defer to ICU4X properties (see: https://docs.rs/icu/latest/icu/properties/props/struct.Emoji.html).
        let is_emoji = matches!(ch as u32, 0x1F600..=0x1F64F | 0x1F300..=0x1F5FF | 0x1F680..=0x1F6FF | 0x2600..=0x26FF | 0x2700..=0x27BF);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "`len_utf8` is between 1 and 4 inclusive"
        )]
        let len_utf8 = ch.len_utf8() as u8;
        Self((len_utf8 - 1) | if is_emoji { Self::EMOJI } else { 0 })
    }

    #[inline(always)]
    pub(crate) const fn with_grapheme_start(mut self, set: bool) -> Self {
        self.0 = self.0 & !Self::GRAPHEME_START | if set { Self::GRAPHEME_START } else { 0 };
        self
    }

    #[inline(always)]
    pub(crate) const fn with_word_boundary(mut self, set: bool) -> Self {
        self.0 = self.0 & !Self::WORD_BOUNDARY | if set { Self::WORD_BOUNDARY } else { 0 };
        self
    }

    #[inline(always)]
    pub(crate) const fn with_line_break_opportunity(mut self, set: bool) -> Self {
        self.0 = self.0 & !Self::LINE_BREAK_OPPORTUNITY
            | if set { Self::LINE_BREAK_OPPORTUNITY } else { 0 };
        self
    }

    #[inline(always)]
    const fn len_utf8(self) -> usize {
        (self.0 & Self::LEN_UTF8_MASK) as usize + 1
    }

    #[inline(always)]
    const fn is_line_break_opportunity(self) -> bool {
        self.0 & Self::LINE_BREAK_OPPORTUNITY != 0
    }

    #[inline(always)]
    const fn is_word_boundary(self) -> bool {
        self.0 & Self::WORD_BOUNDARY != 0
    }

    #[inline(always)]
    const fn is_emoji(self) -> bool {
        self.0 & Self::EMOJI != 0
    }

    #[inline(always)]
    const fn is_grapheme_start(self) -> bool {
        self.0 & Self::GRAPHEME_START != 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShapedClusterFlags(u16);

impl ShapedClusterFlags {
    const GLYPH_LEN_MASK: u16 = 0x00FF;
    const INLINE_GLYPH: u16 = 1 << 8;
    const GRAPHEME_START: u16 = 1 << 9;
    const SAFE_TO_BREAK_BEFORE: u16 = 1 << 10;

    #[inline(always)]
    pub(crate) const fn new(glyph_len: u8) -> Self {
        Self(glyph_len as u16)
    }

    #[inline(always)]
    pub(crate) const fn with_inline_glyph(mut self, set: bool) -> Self {
        self.0 = self.0 & !Self::INLINE_GLYPH | if set { Self::INLINE_GLYPH } else { 0 };
        self
    }

    #[inline(always)]
    pub(crate) const fn with_grapheme_start(mut self, set: bool) -> Self {
        self.0 = self.0 & !Self::GRAPHEME_START | if set { Self::GRAPHEME_START } else { 0 };
        self
    }

    #[inline(always)]
    pub(crate) const fn with_safe_to_break_before(mut self, set: bool) -> Self {
        self.0 =
            self.0 & !Self::SAFE_TO_BREAK_BEFORE | if set { Self::SAFE_TO_BREAK_BEFORE } else { 0 };
        self
    }

    #[inline(always)]
    const fn glyph_len(self) -> u8 {
        (self.0 & Self::GLYPH_LEN_MASK) as u8
    }

    #[inline(always)]
    const fn has_inline_glyph(self) -> bool {
        self.0 & Self::INLINE_GLYPH != 0
    }

    #[inline(always)]
    const fn is_grapheme_start(self) -> bool {
        self.0 & Self::GRAPHEME_START != 0
    }

    #[inline(always)]
    const fn is_safe_to_break_before(self) -> bool {
        self.0 & Self::SAFE_TO_BREAK_BEFORE != 0
    }
}

/// A span of characters and the glyphs they shaped into.
///
/// Shaping may reorder, compose, decompose, etc.; there is no finer-grained correspondence between
/// characters and glyphs. For example, a base letter with a combining mark may shape into a single
/// glyph, a single character may shape into multiple glyphs, and a ligature may combine multiple
/// characters into shared glyphs.
///
/// Shaped cluster boundaries are not necessarily [`Grapheme`][crate::Grapheme] boundaries. The
/// shared boundaries are encoded by [`Atom`][crate::Atom]s.
///
/// For more information about clusters, see [HarfBuzz's documentation][harfbuzz].
///
/// [harfbuzz]: https://harfbuzz.github.io/working-with-harfbuzz-clusters.html
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ShapedCluster {
    /// The characters of this cluster, as a range into
    /// [`ShapedText::characters`](crate::ShapedText::characters).
    //
    // TODO: this currently stores the full range, but perhaps we could store only the start index.
    // See <https://github.com/linebender/parley/pull/715#discussion_r3693794119>.
    pub(crate) chars_range: (u32, u32),

    /// Style index for this cluster.
    pub style_index: u16,

    /// The index into the glyph array where this cluster's glyphs start.
    ///
    /// If [`Self::has_inline_glyph`] is `true`, this is a glyph identifier instead. For more, see
    /// the documentation on that method.
    pub glyph_offset: u32,

    // /// Number of glyphs in this cluster (0xFF = single glyph stored inline)
    // pub glyph_len: u8,
    pub(crate) flags: ShapedClusterFlags,

    /// Advance width for this cluster
    pub advance: f32,
}

impl ShapedCluster {
    /// The characters of this cluster, as a range into [`ShapedText::characters`](crate::ShapedText::characters).
    ///
    /// This is also the range of character positions in the source text (in the Unicode scalar
    /// value sense).
    #[inline(always)]
    pub fn chars_range(&self) -> Range<u32> {
        self.chars_range.0..self.chars_range.1
    }

    /// The number of glyphs of this cluster.
    #[inline(always)]
    pub fn glyph_len(self) -> u8 {
        if self.has_inline_glyph() {
            1
        } else {
            self.flags.glyph_len()
        }
    }

    /// Whether this cluster's glyph is stored inline in [`Self::glyph_offset`].
    ///
    /// This is only possible if [`Self::glyph_len`] is one, and the glyph has no offset.
    /// [`Self::glyph_offset`] then encodes the glyph identifier rather than an index into the glyph
    /// array. The glyph's advance then is this cluster's advance.
    #[inline(always)]
    pub fn has_inline_glyph(self) -> bool {
        self.flags.has_inline_glyph()
    }

    /// Whether this shaped cluster's logical start also starts a grapheme.
    #[inline(always)]
    pub fn is_grapheme_start(self) -> bool {
        self.flags.is_grapheme_start()
    }

    /// Whether breaking logically before this shaped cluster requires reshaping.
    ///
    /// Note that if this shaped cluster does not start a grapheme (see
    /// [`Self::is_grapheme_start`]), you have to reshape regardless of this value.
    #[inline(always)]
    pub fn is_safe_to_break_before(self) -> bool {
        self.flags.is_safe_to_break_before()
    }

    /// The number of graphemes this cluster overlaps.
    pub(crate) fn graphemes_overlapped(&self, characters: &[Character]) -> u32 {
        let start = self.chars_range().start as usize + 1;
        let end = self.chars_range().end as usize;
        let mut graphemes = 1;
        for character in &characters[start..end] {
            graphemes += u32::from(character.is_grapheme_start());
        }
        graphemes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_flags() {
        for (ch, len, emoji) in [
            ('a', 1, false),
            (' ', 1, false),
            ('\t', 1, false),
            ('\r', 1, false),
            ('\n', 1, false),
            ('\u{00e9}', 2, false),
            ('\u{2028}', 3, false),
            ('\u{2600}', 3, true),
            ('\u{1F600}', 4, true),
        ] {
            let flags = CharacterFlags::new(ch);
            assert_eq!(flags.len_utf8(), len, "{ch:?}");
            assert_eq!(flags.is_emoji(), emoji, "{ch:?}");
            assert!(!flags.is_grapheme_start(), "{ch:?}");
            assert!(!flags.is_word_boundary(), "{ch:?}");
            assert!(!flags.is_line_break_opportunity(), "{ch:?}");

            let flags = flags
                .with_grapheme_start(true)
                .with_word_boundary(true)
                .with_line_break_opportunity(true);
            assert_eq!(flags.len_utf8(), len, "{ch:?}");
            assert_eq!(flags.is_emoji(), emoji, "{ch:?}");
            assert!(flags.is_grapheme_start(), "{ch:?}");
            assert!(flags.is_word_boundary(), "{ch:?}");
            assert!(flags.is_line_break_opportunity(), "{ch:?}");

            let flags = flags
                .with_grapheme_start(false)
                .with_word_boundary(false)
                .with_line_break_opportunity(false);
            assert_eq!(flags.len_utf8(), len, "{ch:?}");
            assert_eq!(flags.is_emoji(), emoji, "{ch:?}");
            assert!(!flags.is_grapheme_start(), "{ch:?}");
            assert!(!flags.is_word_boundary(), "{ch:?}");
            assert!(!flags.is_line_break_opportunity(), "{ch:?}");
        }
    }

    #[test]
    fn character_size() {
        assert_eq!(size_of::<Character>(), 8);
    }
}
