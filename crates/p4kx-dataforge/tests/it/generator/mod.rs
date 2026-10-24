// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Random valid Queries for the parser tests.

/// A small deterministic generator, so a failure repeats with its seed.
pub(crate) struct Random(pub(crate) u64);

impl Random {
    pub(crate) const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub(crate) fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap_or(1)).unwrap_or(0)
    }

    pub(crate) fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items
            .get(self.below(items.len()))
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
}

pub(crate) const NAMES: [&str; 12] = [
    "mass",
    "Size",
    "_x1",
    "select",
    "FROM",
    "at",
    "Where",
    "a b",
    "Größe",
    "x.y",
    "\"q\"",
    "EntityClassDefinition",
];

pub(crate) const SPACES: [&str; 5] = [" ", "  ", "\n", "\t ", " \u{3000}"];

pub(crate) fn keyword(random: &mut Random, word: &str) -> String {
    match random.below(3) {
        0 => word.to_owned(),
        1 => word.to_ascii_uppercase(),
        _ => word
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
            .collect(),
    }
}

pub(crate) fn name(random: &mut Random) -> String {
    let name = random.pick(&NAMES);
    let bare = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && ![
            "from",
            "at",
            "referencing",
            "select",
            "where",
            "and",
            "or",
            "not",
        ]
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(name));
    if bare && random.chance(70) {
        name.to_owned()
    } else {
        format!("`{name}`")
    }
}

/// Writes a random path of up to four segments: dotted fields, `->` fields, indices, `[*]` and
/// types, with random whitespace around the arrows, brackets and dots.
pub(crate) fn path(random: &mut Random) -> String {
    let mut text = name(random);
    for _ in 0..random.below(5) {
        let gap = if random.chance(20) {
            random.pick(&SPACES)
        } else {
            ""
        };
        text.push_str(gap);
        let inner = match random.below(5) {
            0 | 4 => {
                text.push_str(if random.chance(50) { "." } else { "->" });
                text.push_str(gap);
                text.push_str(&name(random));
                continue;
            }
            1 => random.pick(&["0", "7", "007", "4294967295"]).to_owned(),
            2 => "*".to_owned(),
            _ => name(random),
        };
        for part in ["[", gap, &inner, gap, "]"] {
            text.push_str(part);
        }
    }
    text
}

/// Writes a random valid Query with random keyword case and whitespace.
pub(crate) fn random_query(random: &mut Random) -> String {
    let mut text = String::new();
    let has_from = random.chance(60);
    if has_from {
        text.push_str(&keyword(random, "from"));
        text.push_str(random.pick(&SPACES));
        text.push_str(&name(random));
    }
    if !has_from || random.chance(50) {
        let glob = random.pick(&["*/ships/*", "", "libs\\foundry", "?a*", "Größe ö", "a\"\"b"]);
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "at"));
        text.push_str(random.pick(&SPACES));
        text.push('"');
        text.push_str(glob);
        text.push('"');
    }
    if random.chance(30) {
        let target = random.pick(&["libs/ships/avenger", "00000000-0000-0000-0000-000000000001"]);
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "referencing"));
        text.push_str(random.pick(&SPACES));
        text.push_str(&["\"", target, "\""].concat());
    }
    if random.chance(70) {
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "select"));
        for column in 0..=random.below(3) {
            if column > 0 {
                text.push_str(random.pick(&SPACES));
                text.push(',');
            }
            text.push_str(random.pick(&SPACES));
            text.push_str(&path(random));
        }
    }
    if random.chance(60) {
        text.push_str(random.pick(&SPACES));
        text.push_str(&keyword(random, "where"));
        text.push_str(random.pick(&SPACES));
        text.push_str(&random_condition(random, 0));
    }
    text.push_str(random.pick(&SPACES));
    text
}

/// Writes a random valid condition nested at most three levels.
pub(crate) fn random_condition(random: &mut Random, depth: usize) -> String {
    let space = |random: &mut Random| random.pick(&SPACES).to_owned();
    let mut text = String::new();
    for part in 0..=random.below(3) {
        if part > 0 {
            let joint = if random.chance(50) { "and" } else { "or" };
            text += &space(random);
            text += &keyword(random, joint);
            text += &space(random);
        }
        if random.chance(20) {
            text += &keyword(random, "not");
            text += &space(random);
        }
        if depth < 3 && random.chance(25) {
            text.push('(');
            text += &random_condition(random, depth + 1);
            text.push(')');
            continue;
        }
        text += &path(random);
        let operator = random.pick(&[
            "=",
            "!=",
            "<",
            "<=",
            ">",
            ">=",
            "contains",
            "starts with",
            "like",
        ]);
        let literal = if ["contains", "starts with", "like"].contains(&operator) {
            random.pick(&["\"x\"", "\"\"", "\"a\"\"b\"", "\"*?\""])
        } else {
            random.pick(&["0", "-12", "1.5", "2e-3", "1E+9", "\"s\"", "true", "FALSE"])
        };
        let operator = keyword(random, operator).replace(' ', &space(random));
        text += &space(random);
        text += &operator;
        text += &space(random);
        text += literal;
    }
    text
}
