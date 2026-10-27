// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::BTreeMap;

/// The most characters of one example a summary prints.
const MAX_EXAMPLE_CHARS: usize = 400;

/// The most examples a summary prints.
pub(crate) const MAX_EXAMPLES: usize = 25;

/// The result of comparing one item with its reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Compared {
    Equal,
    /// Every difference is on the allow-list; the first reason.
    Allowed(&'static str),
    /// The first difference not on the allow-list, with its path.
    Different(String),
}

impl Tally {
    pub(crate) fn record(&mut self, key: &str, compared: Compared) {
        match compared {
            Compared::Equal => self.equal(),
            Compared::Allowed(reason) => self.allow(reason),
            Compared::Different(difference) => self.different(format!("{key}: {difference}")),
        }
    }
}

/// The counts of one differential comparison.
///
/// A difference fails the comparison unless the caller recorded it with [`Tally::allow`] under a
/// documented reason.
#[derive(Debug, Default, Clone)]
pub(crate) struct Tally {
    pub(crate) compared: u64,
    pub(crate) equal: u64,
    pub(crate) different: u64,
    pub(crate) allowed: BTreeMap<&'static str, u64>,
    pub(crate) missing_in_reference: u64,
    pub(crate) missing_in_archive: u64,
    examples: Vec<String>,
}

impl Tally {
    pub(crate) const fn equal(&mut self) {
        self.compared += 1;
        self.equal += 1;
    }

    pub(crate) fn different(&mut self, example: String) {
        self.compared += 1;
        self.different += 1;
        self.example(example);
    }

    pub(crate) fn allow(&mut self, reason: &'static str) {
        self.compared += 1;
        *self.allowed.entry(reason).or_default() += 1;
    }

    pub(crate) const fn missing_in_reference(&mut self) {
        self.missing_in_reference += 1;
    }

    pub(crate) fn missing_in_archive(&mut self, example: String) {
        self.missing_in_archive += 1;
        self.example(example);
    }

    fn example(&mut self, mut example: String) {
        if self.examples.len() < MAX_EXAMPLES {
            if let Some((end, _)) = example.char_indices().nth(MAX_EXAMPLE_CHARS) {
                example.truncate(end);
                example.push('…');
            }
            self.examples.push(example);
        }
    }

    #[must_use]
    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.compared += other.compared;
        self.equal += other.equal;
        self.different += other.different;
        self.missing_in_reference += other.missing_in_reference;
        self.missing_in_archive += other.missing_in_archive;
        for (reason, count) in other.allowed {
            *self.allowed.entry(reason).or_default() += count;
        }
        for example in other.examples {
            self.example(example);
        }
        self
    }

    #[must_use]
    pub(crate) const fn passes(&self) -> bool {
        self.different == 0 && self.missing_in_archive == 0
    }

    #[must_use]
    pub(crate) fn examples(&self) -> &[String] {
        &self.examples
    }

    #[must_use]
    pub(crate) fn summary(&self, name: &str) -> String {
        let head = format!(
            "{name}: {} compared, {} equal, {} different, {} allowed, {} missing in the reference, {} missing in the Archive",
            self.compared,
            self.equal,
            self.different,
            self.allowed.values().sum::<u64>(),
            self.missing_in_reference,
            self.missing_in_archive
        );
        let reasons = self
            .allowed
            .iter()
            .map(|(reason, count)| format!("  allowed {count}: {reason}"));
        let examples = self.examples.iter().map(|example| format!("  {example}"));
        std::iter::once(head)
            .chain(reasons)
            .chain(examples)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Prints the summary and fails when a difference is not on the allow-list.
    pub(crate) fn finish(&self, name: &str) -> Result<(), String> {
        let summary = self.summary(name);
        super::report(&summary);
        if self.passes() { Ok(()) } else { Err(summary) }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_EXAMPLES, Tally};

    #[test]
    fn at_most_the_example_limit_is_kept_and_a_difference_fails() {
        let mut tally = Tally::default();
        for index in 0..MAX_EXAMPLES * 2 {
            tally.different(format!("example {index}"));
        }
        assert_eq!(tally.examples().len(), MAX_EXAMPLES, "capped examples");
        assert!(!tally.passes(), "a difference fails");
    }

    #[test]
    fn an_allowed_difference_and_a_missing_reference_file_do_not_fail() {
        let mut tally = Tally::default();
        tally.equal();
        tally.allow("a documented reason");
        tally.missing_in_reference();
        assert!(tally.passes(), "{}", tally.summary("t"));
        assert_eq!(tally.compared, 2, "equal and allowed are compared");
    }

    #[test]
    fn a_file_missing_in_the_archive_fails() {
        let mut tally = Tally::default();
        tally.missing_in_archive("only in the reference".to_owned());
        assert!(!tally.passes(), "missing in the Archive fails");
    }

    #[test]
    fn merging_adds_counts_and_keeps_the_cap() {
        let mut left = Tally::default();
        let mut right = Tally::default();
        for index in 0..MAX_EXAMPLES {
            left.different(format!("l{index}"));
            right.different(format!("r{index}"));
        }
        let merged = left.merge(right);
        assert_eq!(merged.different, 2 * MAX_EXAMPLES as u64, "counts add");
        assert_eq!(merged.examples().len(), MAX_EXAMPLES, "cap holds");
    }
}
