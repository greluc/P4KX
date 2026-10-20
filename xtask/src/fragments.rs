//! Changelog fragments: one Markdown file per user-visible change in `changelog.d/`.
//!
//! A fragment starts with a `### <Section>` line and holds list items. `release` folds them and the
//! entries under `## [Unreleased]` into the section of a version.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::changelog::{FILE, ORDER, read};

const DIR: &str = "changelog.d";
const README: &str = "README.md";

type Buckets = [Vec<String>; 6];

/// Returns a message for every fragment with a bad name, section or list.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let mut violations = Vec::new();
    for name in names(root)? {
        if !valid_name(&name) {
            violations.push(format!(
                "{DIR}/{name}: the name must be `<issue>.md` or `<issue>-<suffix>.md`"
            ));
        }
        let text = fs::read_to_string(root.join(DIR).join(&name))
            .with_context(|| format!("cannot read {DIR}/{name}"))?;
        if let Err(message) = parse(&text) {
            violations.push(format!("{DIR}/{name}: {message}"));
        }
    }
    Ok(violations)
}

/// Writes the section of `version` dated `date` into `CHANGELOG.md` from the fragments and the
/// entries under `## [Unreleased]`, leaves an empty `## [Unreleased]` and deletes the fragments.
///
/// # Errors
///
/// When a fragment is invalid, `version` or `date` is malformed, the section exists already,
/// `CHANGELOG.md` has no `## [Unreleased]`, or nothing is to be released.
pub(crate) fn release(root: &Path, version: &str, date: &str) -> Result<()> {
    if !valid_version(version) {
        bail!("`{version}` is not a version like 1.2.3");
    }
    if !valid_date(date) {
        bail!("`{date}` is not a date like 2026-10-20");
    }
    let violations = check(root)?;
    if !violations.is_empty() {
        bail!("fix the fragments first:\n{}", violations.join("\n"));
    }
    let text = read(root)?;
    let heading = format!("## [{version}]");
    if text
        .lines()
        .any(|line| line == heading || line.starts_with(&format!("{heading} ")))
    {
        bail!("{FILE} has a section for {version} already");
    }
    let (start, body_start, end) = unreleased(&text)?;
    let mut buckets = Buckets::default();
    let body = text
        .get(body_start..end)
        .context("the offsets are out of range")?;
    let head = text.get(..start).context("the offsets are out of range")?;
    let tail = text.get(end..).context("the offsets are out of range")?;
    collect_unreleased(body, &mut buckets)?;
    let names = names(root)?;
    for name in &names {
        let text = fs::read_to_string(root.join(DIR).join(name))
            .with_context(|| format!("cannot read {DIR}/{name}"))?;
        let (rank, items) = parse(&text).map_err(anyhow::Error::msg)?;
        add(&mut buckets, rank, items);
    }
    let mut section = format!("## [{version}] - {date}\n\n");
    let mut any = false;
    for (name, blocks) in ORDER.iter().zip(&buckets) {
        if !blocks.is_empty() {
            any = true;
            write!(section, "### {name}\n\n{}\n\n", blocks.join("\n"))?;
        }
    }
    if !any {
        bail!("nothing to release: no fragments and nothing under `## [Unreleased]`");
    }
    let released = format!("{head}## [Unreleased]\n\n{section}{tail}");
    fs::write(root.join(FILE), released).with_context(|| format!("cannot write {FILE}"))?;
    for name in &names {
        fs::remove_file(root.join(DIR).join(name))
            .with_context(|| format!("cannot delete {DIR}/{name}"))?;
    }
    Ok(())
}

/// Returns today's date in UTC as `YYYY-MM-DD`.
pub(crate) fn today() -> Result<String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("the clock is before 1970")?
        .as_secs();
    let days = i64::try_from(seconds / 86_400).context("the clock is out of range")?;
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn names(root: &Path) -> Result<Vec<String>> {
    let directory = root.join(DIR);
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&directory).with_context(|| format!("cannot read {DIR}"))? {
        let entry = entry.with_context(|| format!("cannot read {DIR}"))?;
        if entry.file_type().is_ok_and(|kind| kind.is_file())
            && let Some(name) = entry.file_name().to_str()
            && name != README
        {
            names.push(name.to_owned());
        }
    }
    names.sort_by_cached_key(|name| sort_key(name));
    Ok(names)
}

fn sort_key(name: &str) -> (u64, String) {
    let issue = name
        .split(['-', '.'])
        .next()
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(u64::MAX);
    (issue, name.to_owned())
}

fn valid_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    let (issue, suffix) = match stem.split_once('-') {
        Some((issue, suffix)) => (issue, Some(suffix)),
        None => (stem, None),
    };
    !issue.is_empty()
        && issue.bytes().all(|byte| byte.is_ascii_digit())
        && suffix.is_none_or(|suffix| {
            !suffix.is_empty()
                && !suffix.starts_with('-')
                && !suffix.ends_with('-')
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn valid_version(version: &str) -> bool {
    let (core, _) = version.split_once(['-', '+']).unwrap_or((version, ""));
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// Returns the rank in `ORDER` and the trimmed list of a fragment.
fn parse(text: &str) -> Result<(usize, String), String> {
    let mut lines = text.lines();
    let first = lines
        .by_ref()
        .find(|line| !line.trim().is_empty())
        .ok_or("is empty")?;
    let name = first
        .trim_end()
        .strip_prefix("### ")
        .ok_or("must start with a `### <Section>` line")?
        .trim();
    let rank = ORDER
        .iter()
        .position(|known| *known == name)
        .ok_or_else(|| format!("has the unknown section `{name}`; use one of {ORDER:?}"))?;
    let mut items = 0;
    let mut body = Vec::new();
    for line in lines {
        let line = line.trim_end();
        if line.starts_with('#') {
            return Err(format!("has the heading `{line}` inside the items"));
        }
        if line.starts_with("- ") {
            items += 1;
        } else if !line.is_empty() && !line.starts_with(' ') {
            return Err(format!("has text outside a list item: `{line}`"));
        }
        body.push(line);
    }
    if items == 0 {
        return Err("has no list item".to_owned());
    }
    Ok((rank, body.join("\n").trim().to_owned()))
}

/// Returns the offsets of the `## [Unreleased]` heading, of the text below it and of the next
/// `## ` heading or the end.
fn unreleased(text: &str) -> Result<(usize, usize, usize)> {
    let mut offset = 0;
    let mut found: Option<(usize, usize)> = None;
    for line in text.split_inclusive('\n') {
        let end = offset + line.len();
        match found {
            Some((start, body)) if line.starts_with("## ") => return Ok((start, body, offset)),
            None if line.trim_end() == "## [Unreleased]" => found = Some((offset, end)),
            _ => {}
        }
        offset = end;
    }
    match found {
        Some((start, body)) => Ok((start, body, text.len())),
        None => bail!("{FILE} has no `## [Unreleased]` section"),
    }
}

fn add(buckets: &mut Buckets, rank: usize, items: String) {
    if let Some(bucket) = buckets.get_mut(rank) {
        bucket.push(items);
    }
}

fn collect_unreleased(body: &str, buckets: &mut Buckets) -> Result<()> {
    let mut current: Option<(usize, Vec<&str>)> = None;
    let mut flush = |current: Option<(usize, Vec<&str>)>| {
        if let Some((rank, lines)) = current {
            let items = lines.join("\n").trim().to_owned();
            if !items.is_empty() {
                add(buckets, rank, items);
            }
        }
    };
    for line in body.lines() {
        if let Some(name) = line.trim_end().strip_prefix("### ") {
            let name = name.trim();
            let Some(rank) = ORDER.iter().position(|known| *known == name) else {
                bail!("`## [Unreleased]` has the unknown section `### {name}`");
            };
            flush(current.take());
            current = Some((rank, Vec::new()));
        } else if let Some((_, lines)) = current.as_mut() {
            lines.push(line);
        } else if !line.trim().is_empty() {
            bail!("`## [Unreleased]` has text before its first `### ` heading");
        }
    }
    flush(current);
    Ok(())
}

#[cfg(test)]
mod tests;
