use std::ops::Range;

/// Which Entries of one Archive or container are checked, indexed by id, and how many.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Checks {
    marks: Vec<bool>,
    count: usize,
}

impl Checks {
    /// Creates the marks of `len` Entries, none checked.
    pub(crate) fn new(len: usize) -> Self {
        Self {
            marks: vec![false; len],
            count: 0,
        }
    }

    /// Returns the marks, indexed by id.
    pub(crate) fn marks(&self) -> &[bool] {
        &self.marks
    }

    /// Returns whether the Entry `id` is checked.
    pub(crate) fn is_checked(&self, id: u32) -> bool {
        self.marks.get(id as usize).copied().unwrap_or(false)
    }

    /// Checks or unchecks the Entry `id`; returns whether its mark changed.
    pub(crate) fn set(&mut self, id: u32, checked: bool) -> bool {
        let Some(slot) = self.marks.get_mut(id as usize) else {
            return false;
        };
        if *slot == checked {
            return false;
        }
        *slot = checked;
        if checked {
            self.count += 1;
        } else {
            self.count -= 1;
        }
        true
    }

    /// Returns the number of checked Entries.
    pub(crate) const fn count(&self) -> usize {
        self.count
    }

    /// Returns the number of checked Entries among the ids in `range`.
    pub(crate) fn count_in(&self, range: Range<u32>) -> usize {
        self.marks
            .get(range.start as usize..range.end as usize)
            .map_or(0, |slots| slots.iter().filter(|&&checked| checked).count())
    }

    /// Returns the ids of the checked Entries in id order.
    pub(crate) fn ids(&self) -> Vec<u32> {
        self.marks
            .iter()
            .enumerate()
            .filter(|&(_, &checked)| checked)
            .filter_map(|(id, _)| u32::try_from(id).ok())
            .collect()
    }
}
