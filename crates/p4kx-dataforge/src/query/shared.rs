// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::DataForge;
use crate::query::reader::{Reader, Select, Step};
use crate::query::sink::Recording;
use crate::query::walk::Place;

/// The columns of one struct's plan whose paths start with the same steps through arrays, so a
/// Record walks those steps once for all of them.
#[derive(Debug, Clone, Default)]
pub(crate) struct Groups {
    prefixes: Box<[usize]>,
    group_of: Box<[Option<usize>]>,
}

/// The prefix walks of one row, made when the first column of a group asks for them; kept from row
/// to row so their buffers are reused.
pub(crate) type Recordings<'f> = Vec<Recording<'f>>;

impl Groups {
    /// Groups the columns by the longest steps each shares with another column, when those steps
    /// fan out over the elements of an array; the result does not depend on the column order.
    pub(crate) fn of(columns: &[Option<Reader>]) -> Self {
        let walks: Vec<Option<&[Step]>> = columns
            .iter()
            .map(|column| match column {
                Some(Reader::Walk(walk)) => Some(&walk.steps[..]),
                _ => None,
            })
            .collect();
        let mut group_of: Vec<Option<usize>> = vec![None; columns.len()];
        let mut found: Vec<(usize, usize, usize)> = Vec::new();
        for (index, steps) in walks.iter().enumerate() {
            let Some(steps) = steps else { continue };
            let shared = walks
                .iter()
                .enumerate()
                .filter(|&(other, _)| other != index)
                .filter_map(|(_, others)| *others)
                .map(|others| common(steps, others))
                .max()
                .unwrap_or(0);
            if !steps.iter().take(shared).any(fans_out) {
                continue;
            }
            let existing = found.iter().position(|&(first, length, _)| {
                length == shared
                    && walks
                        .get(first)
                        .copied()
                        .flatten()
                        .is_some_and(|others| others.get(..shared) == steps.get(..shared))
            });
            let group = existing.unwrap_or_else(|| {
                found.push((index, shared, 0));
                found.len() - 1
            });
            if let Some(entry) = found.get_mut(group) {
                entry.2 += 1;
            }
            if let Some(slot) = group_of.get_mut(index) {
                *slot = Some(group);
            }
        }
        let mut renumbered = vec![None; found.len()];
        let mut prefixes = Vec::new();
        for (group, &(_, length, members)) in found.iter().enumerate() {
            if members >= 2 {
                if let Some(slot) = renumbered.get_mut(group) {
                    *slot = Some(prefixes.len());
                }
                prefixes.push(length);
            }
        }
        for slot in &mut group_of {
            *slot = slot.and_then(|group| renumbered.get(group).copied().flatten());
        }
        Self {
            prefixes: prefixes.into_boxed_slice(),
            group_of: group_of.into_boxed_slice(),
        }
    }

    /// Returns the columns of each group and the number of steps they share, in the order of the
    /// groups' first columns.
    pub(crate) fn groups(&self) -> Vec<(Vec<usize>, usize)> {
        self.prefixes
            .iter()
            .enumerate()
            .map(|(group, &steps)| {
                let columns = self
                    .group_of
                    .iter()
                    .enumerate()
                    .filter(|&(_, of)| *of == Some(group))
                    .map(|(column, _)| column)
                    .collect();
                (columns, steps)
            })
            .collect()
    }

    /// Readies `recordings` for a new row of a plan with these groups, keeping the buffers.
    pub(crate) fn reset(&self, recordings: &mut Recordings<'_>) {
        recordings.resize_with(self.prefixes.len(), Recording::default);
        for recording in recordings.iter_mut() {
            recording.clear();
        }
    }

    /// Returns the prefix walk the column shares and the number of steps it covers, walked now if
    /// no column of its group has asked yet; `None` for a column that is in no group.
    pub(crate) fn prefix_of<'r, 'f>(
        &self,
        column: usize,
        reader: &Reader,
        forge: &'f DataForge<'f>,
        place: Place<'f>,
        recordings: &'r mut Recordings<'f>,
    ) -> Option<(&'r Recording<'f>, usize)> {
        let group = (*self.group_of.get(column)?)?;
        let Reader::Walk(walk) = reader else {
            return None;
        };
        let prefix = *self.prefixes.get(group)?;
        let recording = recordings.get_mut(group)?;
        if !recording.is_filled() {
            recording.fill(walk, forge, place, prefix);
        }
        Some((&*recording, prefix))
    }
}

fn common(left: &[Step], right: &[Step]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

const fn fans_out(step: &Step) -> bool {
    matches!(
        step,
        Step::Array {
            select: Select::All | Select::Type(_),
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::DataType;
    use crate::ValueKind;
    use crate::query::reader::{Column, Elements, Walk};

    fn array(at: u64, select: Select) -> Step {
        Step::Array {
            at,
            elements: Elements::Values(ValueKind::Int32),
            select,
        }
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "the columns of a plan are Options"
    )]
    fn reader(steps: Vec<Step>) -> Option<Reader> {
        Some(Reader::Walk(Arc::new(Walk {
            steps: steps.into_boxed_slice(),
            leaf: Column {
                data_type: DataType::Int32,
                target: 0,
                start: 0,
                end: 4,
            },
            multi: true,
        })))
    }

    fn groups(columns: &[Option<Reader>]) -> (Vec<Option<usize>>, Vec<usize>) {
        let groups = Groups::of(columns);
        (groups.group_of.to_vec(), groups.prefixes.to_vec())
    }

    #[test]
    fn columns_with_the_same_fan_out_steps_form_a_group() {
        let columns = [
            reader(vec![array(0, Select::All)]),
            reader(vec![array(0, Select::All), array(8, Select::Index(1))]),
            reader(vec![array(0, Select::All)]),
        ];

        assert_eq!(groups(&columns), (vec![Some(0), Some(0), Some(0)], vec![1]));
    }

    #[test]
    fn the_prefix_is_the_longest_one_all_members_share() {
        let columns = [
            reader(vec![array(0, Select::All), array(8, Select::All)]),
            reader(vec![array(0, Select::All), array(8, Select::Index(0))]),
        ];

        assert_eq!(groups(&columns), (vec![Some(0), Some(0)], vec![1]));
    }

    #[test]
    fn steps_without_a_fan_out_are_not_shared() {
        let columns = [
            reader(vec![array(0, Select::Index(0))]),
            reader(vec![array(0, Select::Index(0))]),
        ];

        assert_eq!(groups(&columns), (vec![None, None], vec![]));
    }

    #[test]
    fn different_first_steps_and_lone_columns_form_no_group() {
        let columns = [
            reader(vec![array(0, Select::All)]),
            reader(vec![array(8, Select::All)]),
            None,
            reader(vec![array(0, Select::Type(Box::new([1])))]),
        ];

        assert_eq!(groups(&columns), (vec![None; 4], vec![]));
    }

    #[test]
    fn separate_prefixes_make_separate_groups() {
        let columns = [
            reader(vec![array(0, Select::All)]),
            reader(vec![array(8, Select::All)]),
            reader(vec![array(0, Select::All)]),
            reader(vec![array(8, Select::All)]),
        ];

        assert_eq!(
            groups(&columns),
            (vec![Some(0), Some(1), Some(0), Some(1)], vec![1, 1])
        );
    }

    #[test]
    fn a_column_that_leaves_early_does_not_shorten_the_prefix_of_the_others() {
        let columns = [
            reader(vec![array(0, Select::All)]),
            reader(vec![array(0, Select::All), array(8, Select::All)]),
            reader(vec![array(0, Select::All), array(8, Select::All)]),
        ];

        assert_eq!(groups(&columns), (vec![None, Some(0), Some(0)], vec![2]));
    }

    #[test]
    fn the_grouping_does_not_depend_on_the_column_order() {
        let early = || reader(vec![array(0, Select::All)]);
        let deep = || reader(vec![array(0, Select::All), array(8, Select::All)]);
        let plain = || reader(vec![array(0, Select::Index(0))]);
        let first = [plain(), deep(), deep(), early()];
        let second = [deep(), early(), plain(), deep()];

        let sets = |columns: &[Option<Reader>]| {
            let mut sets: Vec<_> = Groups::of(columns)
                .groups()
                .into_iter()
                .map(|(members, steps)| (members.len(), steps))
                .collect();
            sets.sort_unstable();
            sets
        };

        assert_eq!(sets(&first), vec![(2, 2)]);
        assert_eq!(sets(&second), sets(&first));
    }
}
