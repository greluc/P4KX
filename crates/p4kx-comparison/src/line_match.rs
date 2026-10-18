use std::ops::Range;

/// The most edits the Myers search spends on one region without a unique line.
const MAX_EDIT_DISTANCE: usize = 1024;
/// The comparisons a whole line diff may spend before the remaining regions count as replaced.
const WORK_BUDGET: u64 = 1 << 26;

/// Marks the lines of `old` and `new`, given as interned tokens below `distinct`, that the
/// patience diff removes and adds; regions without a line unique on both sides fall back to
/// Myers. Once the work budget is spent, every region still open is marked as replaced, so the
/// result is deterministic and the work bounded.
pub(crate) fn mark_changes(old: &[u32], new: &[u32], distinct: usize) -> Marks {
    let mut matcher = Matcher {
        old,
        new,
        removed: vec![false; old.len()],
        added: vec![false; new.len()],
        counts: vec![Count::default(); distinct],
        touched: Vec::new(),
        frontier: Vec::new(),
        trace: Vec::new(),
        work: 0,
    };
    let mut open = vec![(0..old.len(), 0..new.len())];
    while let Some((a, b)) = open.pop() {
        matcher.region(a, b, &mut open);
    }
    Marks {
        removed: matcher.removed,
        added: matcher.added,
    }
}

/// Which lines of each side a diff removes and adds.
pub(crate) struct Marks {
    pub(crate) removed: Vec<bool>,
    pub(crate) added: Vec<bool>,
}

impl Marks {
    /// Returns whether the old line at `index` is removed.
    pub(crate) fn is_removed(&self, index: usize) -> bool {
        self.removed.get(index).copied().unwrap_or(false)
    }

    /// Returns whether the new line at `index` is added.
    pub(crate) fn is_added(&self, index: usize) -> bool {
        self.added.get(index).copied().unwrap_or(false)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Count {
    old: u32,
    new: u32,
    old_at: usize,
    new_at: usize,
}

struct Matcher<'t> {
    old: &'t [u32],
    new: &'t [u32],
    removed: Vec<bool>,
    added: Vec<bool>,
    counts: Vec<Count>,
    touched: Vec<u32>,
    frontier: Vec<usize>,
    trace: Vec<usize>,
    work: u64,
}

impl Matcher<'_> {
    fn region(
        &mut self,
        mut a: Range<usize>,
        mut b: Range<usize>,
        open: &mut Vec<(Range<usize>, Range<usize>)>,
    ) {
        while a.start < a.end && b.start < b.end && self.same_line(a.start, b.start) {
            a.start += 1;
            b.start += 1;
        }
        while a.start < a.end && b.start < b.end && self.same_line(a.end - 1, b.end - 1) {
            a.end -= 1;
            b.end -= 1;
        }
        self.spend(a.len() + b.len());
        if a.is_empty() || b.is_empty() || self.work > WORK_BUDGET {
            self.replace(a, b);
            return;
        }
        let anchors = self.unique_anchors(&a, &b);
        if anchors.is_empty() {
            self.myers(a, b);
            return;
        }
        let (mut x, mut y) = (a.start, b.start);
        for (anchor_x, anchor_y) in anchors {
            open.push((x..anchor_x, y..anchor_y));
            (x, y) = (anchor_x + 1, anchor_y + 1);
        }
        open.push((x..a.end, y..b.end));
    }

    fn same_line(&self, x: usize, y: usize) -> bool {
        matches!((self.old.get(x), self.new.get(y)), (Some(a), Some(b)) if a == b)
    }

    const fn spend(&mut self, work: usize) {
        self.work = self.work.saturating_add(work as u64);
    }

    fn replace(&mut self, a: Range<usize>, b: Range<usize>) {
        self.removed.get_mut(a).unwrap_or_default().fill(true);
        self.added.get_mut(b).unwrap_or_default().fill(true);
    }

    /// Returns the lines unique on both sides of the region that form the longest run in the same
    /// order on both sides, as `(old, new)` positions.
    fn unique_anchors(&mut self, a: &Range<usize>, b: &Range<usize>) -> Vec<(usize, usize)> {
        let (old, new) = (self.old, self.new);
        for (at, &token) in old.get(a.clone()).unwrap_or_default().iter().enumerate() {
            if let Some(count) = self.count(token) {
                count.old = count.old.saturating_add(1);
                count.old_at = a.start + at;
            }
        }
        for (at, &token) in new.get(b.clone()).unwrap_or_default().iter().enumerate() {
            if let Some(count) = self.count(token) {
                count.new = count.new.saturating_add(1);
                count.new_at = b.start + at;
            }
        }
        let unique: Vec<(usize, usize)> = old
            .get(a.clone())
            .unwrap_or_default()
            .iter()
            .filter_map(|&token| {
                let count = self.counts.get(token as usize)?;
                (count.old == 1 && count.new == 1).then_some((count.old_at, count.new_at))
            })
            .collect();
        for token in self.touched.drain(..) {
            if let Some(count) = self.counts.get_mut(token as usize) {
                *count = Count::default();
            }
        }
        longest_increasing(&unique)
    }

    fn count(&mut self, token: u32) -> Option<&mut Count> {
        let count = self.counts.get_mut(token as usize)?;
        if count.old == 0 && count.new == 0 {
            self.touched.push(token);
        }
        Some(count)
    }

    /// Marks the shortest edit script of the region found by the greedy Myers search, or the
    /// whole region when it needs more than [`MAX_EDIT_DISTANCE`] edits or the budget runs out.
    fn myers(&mut self, old_range: Range<usize>, new_range: Range<usize>) {
        let (old, new) = (
            self.old.get(old_range.clone()).unwrap_or_default(),
            self.new.get(new_range.clone()).unwrap_or_default(),
        );
        let end = (old.len(), new.len());
        let max = MAX_EDIT_DISTANCE.min(end.0 + end.1);
        let offset = max + 1;
        let mut frontier = std::mem::take(&mut self.frontier);
        let mut trace = std::mem::take(&mut self.trace);
        frontier.clear();
        frontier.resize(2 * max + 3, 0);
        trace.clear();
        let found = self.search(old, new, &mut frontier, &mut trace, max);
        if let Some(distance) = found {
            self.backtrack(
                &trace,
                offset,
                distance,
                (old_range.start, new_range.start),
                end,
            );
        } else {
            self.replace(old_range, new_range);
        }
        (self.frontier, self.trace) = (frontier, trace);
    }

    /// Runs the greedy Myers search over `frontier`, keeping each round's frontier in `trace`;
    /// returns the edit distance, or `None` past `max` edits or the work budget.
    fn search(
        &mut self,
        old: &[u32],
        new: &[u32],
        frontier: &mut [usize],
        trace: &mut Vec<usize>,
        max: usize,
    ) -> Option<usize> {
        let end = (old.len(), new.len());
        let offset = max + 1;
        for d in 0..=max {
            trace.extend_from_slice(frontier.get(offset - d..=offset + d).unwrap_or_default());
            for k in (offset - d..=offset + d).step_by(2) {
                let start = next_x(frontier, k, offset, d);
                let mut point = (start, (start + offset).saturating_sub(k));
                while let (Some(left), Some(right)) = (old.get(point.0), new.get(point.1))
                    && left == right
                {
                    point = (point.0 + 1, point.1 + 1);
                }
                self.spend(point.0 - start + 1);
                if let Some(slot) = frontier.get_mut(k) {
                    *slot = point.0;
                }
                if point.0 >= end.0 && point.1 >= end.1 {
                    return Some(d);
                }
            }
            if self.work > WORK_BUDGET {
                return None;
            }
        }
        None
    }

    fn backtrack(
        &mut self,
        trace: &[usize],
        offset: usize,
        distance: usize,
        (old_start, new_start): (usize, usize),
        (mut x, mut y): (usize, usize),
    ) {
        for d in (1..=distance).rev() {
            let base = d * d;
            let row = |k: usize| {
                (k + d)
                    .checked_sub(offset)
                    .and_then(|at| trace.get(base + at))
                    .copied()
                    .unwrap_or(0)
            };
            let k = (x + offset).saturating_sub(y);
            let down =
                k + d == offset || (k != offset + d && row(k.saturating_sub(1)) < row(k + 1));
            let previous_k = if down { k + 1 } else { k.saturating_sub(1) };
            let previous_x = row(previous_k);
            let previous_y = (previous_x + offset).saturating_sub(previous_k);
            let snake = x
                .saturating_sub(previous_x)
                .min(y.saturating_sub(previous_y));
            let (marks, start, edit) = if down {
                (&mut self.added, new_start, y.saturating_sub(snake))
            } else {
                (&mut self.removed, old_start, x.saturating_sub(snake))
            };
            if let Some(line) = edit.checked_sub(1).map(|edited| start + edited)
                && let Some(mark) = marks.get_mut(line)
            {
                *mark = true;
            }
            (x, y) = (previous_x, previous_y);
        }
    }
}

/// Returns the x a path reaching diagonal `k` in round `d` starts its snake at.
fn next_x(frontier: &[usize], k: usize, offset: usize, d: usize) -> usize {
    let at = |k: usize| frontier.get(k).copied().unwrap_or(0);
    if k + d == offset || (k != offset + d && at(k.saturating_sub(1)) < at(k + 1)) {
        at(k + 1)
    } else {
        at(k.saturating_sub(1)) + 1
    }
}

/// Returns the longest subsequence of `pairs`, ordered by their first item, whose second items
/// increase.
fn longest_increasing(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![usize::MAX; pairs.len()];
    for (index, &(_, y)) in pairs.iter().enumerate() {
        let position =
            tails.partition_point(|&tail| pairs.get(tail).is_some_and(|&(_, tail_y)| tail_y < y));
        if let Some(&before) = position.checked_sub(1).and_then(|at| tails.get(at))
            && let Some(slot) = previous.get_mut(index)
        {
            *slot = before;
        }
        if let Some(tail) = tails.get_mut(position) {
            *tail = index;
        } else {
            tails.push(index);
        }
    }
    let mut run = Vec::with_capacity(tails.len());
    let mut at = tails.last().copied().unwrap_or(usize::MAX);
    while let Some(&pair) = pairs.get(at) {
        run.push(pair);
        at = previous.get(at).copied().unwrap_or(usize::MAX);
    }
    run.reverse();
    run
}
