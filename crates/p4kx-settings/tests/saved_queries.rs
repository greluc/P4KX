//! The saved Queries the GUI keeps in the settings file, read as untrusted input.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use p4kx_settings::{
    MAX_SETTINGS_SIZE, NameProblem, SavedQueries, SavedQuery, SavedQueryEdit, Settings,
    SettingsError,
};

type TestResult = Result<(), Box<dyn Error>>;

fn names(queries: &SavedQueries) -> Vec<&str> {
    queries.iter().map(SavedQuery::name).collect()
}

fn load(content: &str) -> Result<Settings, Box<dyn Error>> {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, content)?;
    Ok(Settings::load(&file)?)
}

#[test]
fn a_query_is_saved_renamed_and_removed() -> TestResult {
    let mut queries = SavedQueries::default();

    assert_eq!(
        queries.save("  Ships ", "from Ship select mass"),
        Ok(0),
        "saved"
    );
    assert_eq!(
        queries.save("Items", "from Item select mass"),
        Ok(1),
        "second"
    );
    queries.rename(0, "Heavy ships")?;
    assert!(queries.remove(1), "removed");

    assert_eq!(names(&queries), ["Heavy ships"], "names");
    assert_eq!(
        queries.get(0).map(SavedQuery::text),
        Some("from Ship select mass"),
        "text"
    );
    assert!(!queries.remove(5), "nothing at 5");
    Ok(())
}

#[test]
fn saving_under_a_taken_name_replaces_its_text() {
    let mut queries = SavedQueries::default();

    let first = queries.save("Ships", "from Ship select mass");
    let again = queries.save("Ships", "from Ship select crew");

    assert_eq!((first, again), (Ok(0), Ok(0)), "same place");
    assert_eq!(queries.len(), 1, "one Query");
    assert_eq!(
        queries.get(0).map(SavedQuery::text),
        Some("from Ship select crew"),
        "replaced"
    );
}

#[test]
fn names_and_texts_are_checked() {
    let mut queries = SavedQueries::default();
    let long = "x".repeat(SavedQueries::MAX_NAME_LEN + 1);
    let huge = "x".repeat(SavedQueries::MAX_TEXT_LEN + 1);

    assert_eq!(
        queries.save(" ", "from A"),
        Err(NameProblem::Empty),
        "empty"
    );
    assert_eq!(
        queries.save(&long, "from A"),
        Err(NameProblem::TooLong),
        "long"
    );
    assert_eq!(
        queries.save("A", "  "),
        Err(NameProblem::BadText),
        "no text"
    );
    assert_eq!(queries.save("A", &huge), Err(NameProblem::BadText), "huge");
    let _first = queries.save("A", "from A");
    let _second = queries.save("B", "from B");
    assert_eq!(queries.rename(1, "A"), Err(NameProblem::Taken), "taken");
    assert_eq!(queries.check_rename(0, "A "), None, "its own name");
}

#[test]
fn no_more_than_the_limit_are_saved() {
    let mut queries = SavedQueries::default();
    for n in 0..SavedQueries::LIMIT {
        let _saved = queries.save(&format!("Q{n}"), "from A");
    }

    assert_eq!(
        queries.save("one more", "from A"),
        Err(NameProblem::Full),
        "full"
    );
    assert_eq!(queries.save("Q3", "from B"), Ok(3), "replacing still works");
}

#[test]
fn saved_queries_survive_a_round_trip_next_to_the_other_settings() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut settings = Settings {
        language: Some("en-GB".to_owned()),
        ..Settings::default()
    };
    settings.recent.push(PathBuf::from(r"C:\SC\LIVE\Data.p4k"));
    let _saved = settings
        .queries
        .save("Ships \"quoted\"", "from Ship\nselect `Size-X`");

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "round trip");
    Ok(())
}

#[test]
fn corrupt_entries_are_dropped_and_the_rest_is_kept() -> TestResult {
    let settings = load(
        "language = \"de-DE\"\n\
         [[queries]]\nname = \"Good\"\ntext = \"from A\"\n\
         [[queries]]\nname = 7\ntext = \"from B\"\n\
         [[queries]]\nname = \"   \"\ntext = \"from C\"\n\
         [[queries]]\nname = \"Good\"\ntext = \"from D\"\n\
         [[queries]]\ntext = \"from E\"\n\
         [[queries]]\nname = \"Also good\"\ntext = \"from F\"\n",
    )?;

    assert_eq!(names(&settings.queries), ["Good", "Also good"], "kept");
    assert_eq!(
        settings.language.as_deref(),
        Some("de-DE"),
        "the rest is read"
    );
    Ok(())
}

#[test]
fn a_value_that_is_no_list_reads_as_no_saved_query() -> TestResult {
    for content in [
        "queries = 5\n",
        "queries = \"x\"\n",
        "[queries]\nname = \"A\"\n",
    ] {
        let settings = load(content)?;
        assert!(settings.queries.is_empty(), "{content}");
    }
    Ok(())
}

#[test]
fn entries_beyond_the_limit_are_dropped_on_load() -> TestResult {
    let mut content = String::new();
    for n in 0..SavedQueries::LIMIT + 5 {
        write!(content, "[[queries]]\nname = \"Q{n}\"\ntext = \"from A\"\n")?;
    }

    let settings = load(&content)?;

    assert_eq!(settings.queries.len(), SavedQueries::LIMIT, "capped");
    Ok(())
}

#[test]
fn a_file_larger_than_the_cap_is_refused() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let size = usize::try_from(MAX_SETTINGS_SIZE)? + 1;
    std::fs::write(&file, " ".repeat(size))?;

    assert!(
        matches!(Settings::load(&file), Err(SettingsError::TooLarge { .. })),
        "too large"
    );
    Ok(())
}

#[test]
fn names_with_control_characters_are_refused() {
    let mut queries = SavedQueries::default();

    assert_eq!(
        queries.save("Two\nlines", "from A"),
        Err(NameProblem::Control),
        "line break"
    );
    assert_eq!(
        queries.save("A", "from A\u{7}"),
        Err(NameProblem::BadText),
        "bell in the text"
    );
    assert_eq!(
        queries.save("A", "from A\n\twhere x = 1"),
        Ok(0),
        "tab and line break"
    );
}

#[test]
fn the_total_size_is_budgeted() {
    let mut queries = SavedQueries::default();
    let text = "x".repeat(SavedQueries::MAX_TEXT_LEN);
    let fit = SavedQueries::MAX_TOTAL_LEN / (SavedQueries::MAX_TEXT_LEN + 4);
    for n in 0..fit {
        assert!(queries.save(&format!("Q{n:03}"), &text).is_ok(), "{n} fits");
    }

    assert_eq!(
        queries.check_save("one more", &text),
        Some(NameProblem::Full),
        "over the budget"
    );
    assert_eq!(
        queries.check_save("Q001", "from A"),
        None,
        "replacing shrinks it"
    );
    assert!(
        queries.total_len() <= SavedQueries::MAX_TOTAL_LEN,
        "within budget"
    );
}

#[test]
fn edits_by_name_replay_onto_another_copy() {
    let mut mine = SavedQueries::default();
    let _a = mine.save("A", "from A");
    let mut theirs = mine.clone();
    let _b = theirs.save("B", "from B");

    for edit in [
        SavedQueryEdit::Save {
            name: "C".to_owned(),
            text: "from C".to_owned(),
        },
        SavedQueryEdit::Rename {
            from: "A".to_owned(),
            to: "A2".to_owned(),
        },
        SavedQueryEdit::Delete {
            name: "gone".to_owned(),
        },
    ] {
        theirs.apply(&edit);
    }

    assert_eq!(names(&theirs), ["A2", "B", "C"], "merged by name");
}

#[test]
fn settings_too_large_to_read_again_are_not_written() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut settings = Settings::default();
    let huge = "d".repeat(1024 * 1024);
    for n in 0..17 {
        settings
            .channels
            .add_folder(PathBuf::from(format!("{huge}{n}")));
    }

    assert!(
        matches!(settings.save(&file), Err(SettingsError::TooLarge { .. })),
        "refused"
    );
    assert!(!file.exists(), "nothing written");
    Ok(())
}

#[test]
fn saving_replaces_the_file_and_leaves_no_temporary_file() -> TestResult {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "language = \"de-DE\"\n")?;
    let settings = Settings {
        language: Some("en-GB".to_owned()),
        ..Settings::default()
    };

    settings.save(&file)?;

    assert_eq!(Settings::load(&file)?, settings, "replaced");
    let left: Vec<_> = std::fs::read_dir(folder.path())?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<_, _>>()?;
    assert_eq!(left, ["settings.toml"], "only the settings file");
    Ok(())
}
