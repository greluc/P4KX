// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{Issue, IssueKind};

/// How many animation names an `animation_not_used` issue lists as examples.
const UNUSED_EXAMPLES: usize = 3;

/// Replaces the `animation_not_used` issues by one per reason, in the order the reasons first
/// occur and after the other issues. Reasons starting with `limit` count as one reason.
pub(super) fn summarise(issues: Vec<Issue>, budget: usize) -> Vec<Issue> {
    let mut rest = Vec::with_capacity(issues.len().min(16));
    let mut groups: Vec<Issue> = Vec::new();
    for issue in issues {
        let Issue { path, name, kind } = issue;
        let IssueKind::AnimationNotUsed { reason, .. } = kind else {
            rest.push(Issue { path, name, kind });
            continue;
        };
        let reason = if reason.starts_with("limit") {
            format!("limit: the budget of {budget} values is used up")
        } else {
            reason
        };
        let found = groups.iter_mut().find_map(|group| match &mut group.kind {
            IssueKind::AnimationNotUsed {
                reason: known,
                count,
                examples,
            } if *known == reason => Some((count, examples)),
            _ => None,
        });
        match found {
            Some((count, examples)) => {
                *count = count.saturating_add(1);
                if examples.len() < UNUSED_EXAMPLES {
                    examples.push(name);
                }
            }
            None => groups.push(Issue {
                path: String::new(),
                name: name.clone(),
                kind: IssueKind::AnimationNotUsed {
                    reason,
                    count: 1,
                    examples: vec![name],
                },
            }),
        }
    }
    rest.extend(groups);
    rest
}
