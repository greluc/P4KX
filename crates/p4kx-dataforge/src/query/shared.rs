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

/// The prefix walks of one row, made when the first column of a group asks for them.
pub(crate) type Recordings<'f> = Vec<Option<Recording<'f>>>;

impl Groups {
    /// Groups the columns that begin with the same steps, the longest steps they all share, when
    /// those steps fan out over the elements of an array.
    pub(crate) fn of(columns: &[Option<Reader>]) -> Self {
        let walks: Vec<_> = columns
            .iter()
            .map(|column| match column {
                Some(Reader::Walk(walk)) => Some(&walk.steps[..]),
                _ => None,
            })
            .collect();
        let mut group_of = vec![None; columns.len()];
        let mut prefixes = Vec::new();
        for (index, steps) in walks.iter().enumerate() {
            let Some(steps) = steps else { continue };
            if group_of.get(index).is_some_and(Option::is_some) {
                continue;
            }
            let members: Vec<usize> = walks
                .iter()
                .enumerate()
                .skip(index)
                .filter(|&(other, others)| {
                    group_of.get(other).is_some_and(Option::is_none)
                        && others.is_some_and(|others| others.first() == steps.first())
                })
                .map(|(other, _)| other)
                .collect();
            let shared = members
                .iter()
                .filter_map(|&member| walks.get(member).copied().flatten())
                .fold(steps.len(), |shared, others| {
                    steps
                        .iter()
                        .zip(others)
                        .take(shared)
                        .take_while(|(left, right)| left == right)
                        .count()
                });
            if members.len() < 2 || !steps.iter().take(shared).any(fans_out) {
                continue;
            }
            for member in members {
                if let Some(slot) = group_of.get_mut(member) {
                    *slot = Some(prefixes.len());
                }
            }
            prefixes.push(shared);
        }
        Self {
            prefixes: prefixes.into_boxed_slice(),
            group_of: group_of.into_boxed_slice(),
        }
    }

    /// Returns the cache for the prefix walks of one row, which allocates only for a plan with
    /// groups.
    pub(crate) fn recordings<'f>(&self) -> Recordings<'f> {
        (0..self.prefixes.len()).map(|_| None).collect()
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
        let slot = recordings.get_mut(group)?;
        let recording = slot.get_or_insert_with(|| Recording::of(walk, forge, place, prefix));
        Some((recording, prefix))
    }
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
}
