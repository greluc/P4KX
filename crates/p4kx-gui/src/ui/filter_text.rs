// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Filter, FilterError, MAX_EXCLUSIONS, MAX_FILTER_LEN, MAX_FILTER_WILDCARDS};

use crate::{ExtractionText as T, Texts};

/// Returns the words for a Filter text that is not accepted.
pub(super) fn problem_text(texts: &Texts, problem: &FilterError) -> String {
    let limit = |max: usize| [("max", u64::try_from(max).unwrap_or(u64::MAX).into())];
    match problem {
        FilterError::TooLong { .. } => texts.format(T::FilterTooLong, &limit(MAX_FILTER_LEN)),
        FilterError::TooManyWildcards { .. } => {
            texts.format(T::FilterTooManyWildcards, &limit(MAX_FILTER_WILDCARDS))
        }
        FilterError::TooManyExclusions { .. } => {
            texts.format(T::ExcludeTooMany, &limit(MAX_EXCLUSIONS))
        }
        FilterError::EmptyExclusion => texts.get(T::ExcludeEmpty),
    }
}

/// Returns why the Filter `text` of a search field is refused, in words, or `None` when it is
/// accepted; checks the limits without building the Filter.
pub(super) fn filter_problem(texts: &Texts, text: &str) -> Option<String> {
    Filter::check(text)
        .err()
        .map(|problem| problem_text(texts, &problem))
}
