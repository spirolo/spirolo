//! Source locations, comparable so tables can be sorted deterministically.

use serde::{Deserialize, Serialize};

/// A half-open byte range in a source file, plus the human coordinates of its start.
///
/// Byte offsets are what the parser hands us; line and column are carried alongside
/// so a report never needs the file again. Ordering is by offset first, which is what
/// "sorted by path then span" means everywhere else in this workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Span {
    /// Byte offset of the first byte of the range.
    pub start: u32,
    /// Byte offset one past the last byte of the range.
    pub end: u32,
    /// One-based line of `start`.
    pub line: u32,
    /// One-based UTF-8 column of `start`.
    pub column: u32,
}

impl Span {
    /// A span covering `start..end` with unknown line and column.
    pub const fn new(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            line: 0,
            column: 0,
        }
    }

    /// The byte range `start..end`, with the one-based line and column of `start`
    /// resolved against `source`.
    ///
    /// Columns are counted in bytes, which matches the byte offsets the parser hands
    /// us; a multi-byte character before the span shifts the column by its byte width.
    pub fn locate(source: &str, start: u32, end: u32) -> Self {
        let before = &source[..(start as usize).min(source.len())];
        let line = u32::try_from(before.matches('\n').count() + 1).unwrap_or(u32::MAX);
        let column_bytes = before.rsplit('\n').next().map_or(0, str::len);
        Self {
            start,
            end,
            line,
            column: u32::try_from(column_bytes + 1).unwrap_or(u32::MAX),
        }
    }
}

/// The line starts of one source file, so each span is located by a binary search
/// rather than by counting newlines from the top of the file.
///
/// A collector that locates every identifier reference with [`Span::locate`] rescans the
/// file once per reference; on a large monorepo that rescan was most of `spirolo list` (#153).
/// Build one index per file and place every span through it; the result is exactly
/// [`Span::locate`]'s.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset of the first byte of each line; `line_starts[0]` is 0.
    line_starts: Vec<u32>,
    /// The source length, where an offset past the end is clamped.
    len: u32,
}

impl LineIndex {
    /// Index the line starts of `source`.
    pub fn new(source: &str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(
                source
                    .bytes()
                    .enumerate()
                    .filter(|&(_, byte)| byte == b'\n')
                    .map(|(at, _)| u32::try_from(at + 1).unwrap_or(u32::MAX)),
            )
            .collect();
        Self {
            line_starts,
            len: u32::try_from(source.len()).unwrap_or(u32::MAX),
        }
    }

    /// The byte range `start..end`, with the one-based line and column of `start`, as
    /// [`Span::locate`] computes them against the indexed source.
    pub fn locate(&self, start: u32, end: u32) -> Span {
        let at = start.min(self.len);
        // The number of line starts at or before `at` is the one-based line.
        let line = self
            .line_starts
            .partition_point(|&line_start| line_start <= at);
        let line_start = self.line_starts[line - 1];
        Span {
            start,
            end,
            line: u32::try_from(line).unwrap_or(u32::MAX),
            column: at - line_start + 1,
        }
    }
}
