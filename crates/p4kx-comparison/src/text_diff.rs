use std::ops::Range;

use schemars::JsonSchema;
use serde::Serialize;
use std::collections::HashMap;

use crate::line_match::mark_changes;

/// The number of unchanged lines a unified diff shows around each change.
pub const DEFAULT_CONTEXT: usize = 3;

/// What one line of a hunk is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    /// The line is on both sides.
    Context,
    /// The line is only on the old side.
    Removed,
    /// The line is only on the new side.
    Added,
}

/// One line of a hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffLine<'a> {
    kind: LineKind,
    text: &'a str,
}

impl<'a> DiffLine<'a> {
    /// Returns whether the line is on both sides, removed or added.
    #[must_use]
    pub const fn kind(&self) -> LineKind {
        self.kind
    }

    /// Returns the line with its line terminator; the last line of a side may have none.
    #[must_use]
    pub const fn text(&self) -> &'a str {
        self.text
    }
}

/// One side of a text diff split into lines.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Lines {
    text: String,
    ends: Vec<usize>,
}

impl Lines {
    fn new(text: String) -> Self {
        let ends = text
            .split_inclusive('\n')
            .scan(0, |end, line| {
                *end += line.len();
                Some(*end)
            })
            .collect();
        Self { text, ends }
    }

    const fn len(&self) -> usize {
        self.ends.len()
    }

    fn line(&self, index: usize) -> &str {
        let start = index
            .checked_sub(1)
            .and_then(|previous| self.ends.get(previous))
            .copied()
            .unwrap_or(0);
        self.ends
            .get(index)
            .and_then(|&end| self.text.get(start..end))
            .unwrap_or_default()
    }
}

/// One run of lines replaced by another, as line indices of each side.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Change {
    old: Range<usize>,
    new: Range<usize>,
}

/// Changes close enough to share their context, with the lines shown around them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HunkSpan {
    old: Range<usize>,
    new: Range<usize>,
    changes: Range<usize>,
}

/// The line diff of two texts as hunks of changed lines with their context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiff {
    old: Lines,
    new: Lines,
    changes: Vec<Change>,
    hunks: Vec<HunkSpan>,
}

impl TextDiff {
    /// Diffs `old` against `new` line by line with the patience algorithm, keeping `context`
    /// unchanged lines around each change.
    pub(crate) fn new(old: String, new: String, context: usize) -> Self {
        let (old, new) = (Lines::new(old), Lines::new(new));
        let changes = changes(&old, &new);
        let hunks = group(&changes, context, old.len());
        Self {
            old,
            new,
            changes,
            hunks,
        }
    }

    /// Returns the hunks in line order.
    pub fn hunks(&self) -> impl ExactSizeIterator<Item = Hunk<'_>> {
        self.hunks.iter().map(|span| Hunk { diff: self, span })
    }

    /// Returns whether the texts differ.
    #[must_use]
    pub const fn has_differences(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Returns the old text.
    #[must_use]
    pub fn old_text(&self) -> &str {
        &self.old.text
    }

    /// Returns the new text.
    #[must_use]
    pub fn new_text(&self) -> &str {
        &self.new.text
    }
}

/// Interns the lines of both sides and returns the runs of removed and added lines.
fn changes(old: &Lines, new: &Lines) -> Vec<Change> {
    let mut ids: HashMap<&str, u32> = HashMap::with_capacity(old.len().max(new.len()));
    let (old_tokens, new_tokens) = (intern(&mut ids, old), intern(&mut ids, new));
    let (removed, added) = mark_changes(&old_tokens, &new_tokens, ids.len());
    let mut changes = Vec::new();
    let (mut x, mut y) = (0, 0);
    let is = |marks: &[bool], at: usize| marks.get(at).copied().unwrap_or(false);
    while x < removed.len() || y < added.len() {
        if is(&removed, x) || is(&added, y) {
            let (old_start, new_start) = (x, y);
            while is(&removed, x) {
                x += 1;
            }
            while is(&added, y) {
                y += 1;
            }
            changes.push(Change {
                old: old_start..x,
                new: new_start..y,
            });
        } else {
            x += 1;
            y += 1;
        }
    }
    changes
}

/// Returns the token of every line, giving each distinct line the next free token.
fn intern<'t>(ids: &mut HashMap<&'t str, u32>, lines: &'t Lines) -> Vec<u32> {
    (0..lines.len())
        .map(|index| {
            let next = u32::try_from(ids.len()).unwrap_or(u32::MAX);
            *ids.entry(lines.line(index)).or_insert(next)
        })
        .collect()
}

/// Groups `changes` into hunks whose context does not overlap.
fn group(changes: &[Change], context: usize, old_len: usize) -> Vec<HunkSpan> {
    let mut hunks: Vec<HunkSpan> = Vec::new();
    for (index, change) in changes.iter().enumerate() {
        let lead = context.min(change.old.start);
        let trail = context.min(old_len.saturating_sub(change.old.end));
        let old_end = change.old.end.saturating_add(trail);
        let new_end = change.new.end.saturating_add(trail);
        if let Some(last) = hunks.last_mut()
            && change.old.start.saturating_sub(lead) <= last.old.end
        {
            last.old.end = old_end;
            last.new.end = new_end;
            last.changes.end = index + 1;
        } else {
            hunks.push(HunkSpan {
                old: change.old.start - lead..old_end,
                new: change.new.start.saturating_sub(lead)..new_end,
                changes: index..index + 1,
            });
        }
    }
    hunks
}

/// One hunk of a [`TextDiff`]: changed lines with the unchanged lines around them.
#[derive(Debug, Clone, Copy)]
pub struct Hunk<'a> {
    diff: &'a TextDiff,
    span: &'a HunkSpan,
}

impl<'a> Hunk<'a> {
    /// Returns the zero-based line range the hunk covers on the old side.
    #[must_use]
    pub fn old_lines(&self) -> Range<usize> {
        self.span.old.clone()
    }

    /// Returns the zero-based line range the hunk covers on the new side.
    #[must_use]
    pub fn new_lines(&self) -> Range<usize> {
        self.span.new.clone()
    }

    /// Returns the first line of the old side as a unified diff header writes it: one-based, or
    /// the line before the hunk when it covers no old line.
    #[must_use]
    pub const fn old_start(&self) -> usize {
        unified_start(&self.span.old)
    }

    /// Returns the first line of the new side as a unified diff header writes it: one-based, or
    /// the line before the hunk when it covers no new line.
    #[must_use]
    pub const fn new_start(&self) -> usize {
        unified_start(&self.span.new)
    }

    /// Returns the lines of the hunk in unified-diff order: removed lines before added ones.
    pub fn lines(&self) -> impl Iterator<Item = DiffLine<'a>> + use<'a> {
        let diff = self.diff;
        let changes = diff
            .changes
            .get(self.span.changes.clone())
            .unwrap_or_default();
        let tail = changes.last().map_or(self.span.old.start, |c| c.old.end)..self.span.old.end;
        let mut cursor = self.span.old.start;
        let old = move |kind| {
            move |index| DiffLine {
                kind,
                text: diff.old.line(index),
            }
        };
        changes
            .iter()
            .flat_map(move |change| {
                let lead = cursor..change.old.start;
                cursor = change.old.end;
                lead.map(old(LineKind::Context))
                    .chain(change.old.clone().map(old(LineKind::Removed)))
                    .chain(change.new.clone().map(move |index| DiffLine {
                        kind: LineKind::Added,
                        text: diff.new.line(index),
                    }))
            })
            .chain(tail.map(old(LineKind::Context)))
    }
}

const fn unified_start(lines: &Range<usize>) -> usize {
    if lines.start == lines.end {
        lines.start
    } else {
        lines.start.saturating_add(1)
    }
}
