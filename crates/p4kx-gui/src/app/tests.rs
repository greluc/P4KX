use std::path::PathBuf;

use p4kx_channels::SearchPaths;
use p4kx_settings::{ExtractionTextures, ManualChannels, RecentArchives, SavedQueries, Settings};

use super::{App, remember};
use crate::{ChannelList, Language, Model, RecordTextChoice, TextLanguage};

#[test]
fn the_language_stays_unset_until_the_user_chooses_one() {
    let mut settings = Settings::default();

    remember(&mut settings, &Model::new(Language::German), false, None);
    let unchosen = settings.language.clone();
    remember(
        &mut settings,
        &Model::new(Language::BritishEnglish),
        true,
        None,
    );

    assert_eq!(unchosen, None, "follows the system");
    assert_eq!(settings.language.as_deref(), Some("en-GB"), "chosen");
}

#[test]
fn the_record_text_choice_is_stored_and_stays_unset_until_made() {
    let mut model = Model::new(Language::German);
    let mut settings = Settings::default();

    remember(&mut settings, &model, false, None);
    let unchosen = settings.record_language.clone();
    model.set_record_text_choice(Some(RecordTextChoice::Language(TextLanguage::French)));
    remember(&mut settings, &model, false, None);
    let french = settings.record_language.clone();
    model.set_record_text_choice(Some(RecordTextChoice::KeysOnly));
    remember(&mut settings, &model, false, None);

    assert_eq!(unchosen, None, "follows the interface");
    assert_eq!(french.as_deref(), Some("fr"), "a language");
    assert_eq!(
        settings.record_language.as_deref(),
        Some("none"),
        "keys only"
    );
}

#[test]
fn manual_entries_join_those_already_in_the_settings() -> Result<(), std::io::Error> {
    let folder = tempfile::tempdir()?;
    let mut settings = Settings::default();
    settings.channels.add_folder(PathBuf::from(r"D:\Elsewhere"));
    let mut model = Model::new(Language::German);
    model.add_manual(folder.path().to_path_buf());

    remember(&mut settings, &model, false, None);

    assert_eq!(
        settings.channels.folders,
        [PathBuf::from(r"D:\Elsewhere"), folder.path().to_path_buf()],
        "both folders are kept"
    );
    Ok(())
}

#[test]
fn the_extraction_folder_is_stored() {
    let mut model = Model::new(Language::German);
    model.set_extraction_folder(Some(PathBuf::from(r"D:\Exports")));
    let mut settings = Settings::default();

    remember(&mut settings, &model, false, None);

    assert_eq!(
        settings.extraction_folder,
        Some(PathBuf::from(r"D:\Exports")),
        "the folder chosen last"
    );
}

#[test]
fn the_update_check_is_stored() {
    let mut model = Model::new(Language::German);
    model.set_update_check(true);
    let mut settings = Settings::default();

    remember(&mut settings, &model, false, None);

    assert!(settings.update_check, "switched on");
}

#[test]
fn the_conversion_switch_is_stored() {
    let mut model = Model::new(Language::German);
    model.set_extraction_conversion(true);
    let mut settings = Settings::default();

    remember(&mut settings, &model, false, None);

    assert!(settings.extraction_conversion, "switched on");
}

#[test]
fn the_texture_outputs_are_stored() {
    let mut model = Model::new(Language::German);
    let textures = ExtractionTextures {
        merge: false,
        png: true,
        mip: 3,
        keep_raw: true,
    };
    model.set_extraction_textures(textures);
    let mut settings = Settings::default();

    remember(&mut settings, &model, false, None);

    assert_eq!(settings.extraction_textures, textures, "stored");
}

#[test]
fn an_entry_removed_elsewhere_is_not_written_back() {
    let mut model = Model::new(Language::German);
    let mut loaded = ManualChannels::default();
    loaded.add_folder(PathBuf::from(r"D:\Removed"));
    model.set_channels(ChannelList::new(SearchPaths::default(), loaded));
    let mut recent = RecentArchives::default();
    recent.push(PathBuf::from(r"C:\SC\LIVE\Data.p4k"));
    model.set_recent(recent.clone());
    let mut settings = Settings::default();

    remember(&mut settings, &model, false, None);

    assert!(settings.channels.folders.is_empty(), "stays removed");
    assert_eq!(settings.recent, recent, "the recent Archives are stored");
}

fn app_over(file: &std::path::Path) -> App {
    App::new(Model::new(Language::German), Some(file.to_path_buf()), None)
}

fn save(app: &mut App, name: &str, text: &str) {
    let query = app.model.query_mut();
    query.set_text(text);
    query.begin_save();
    if let Some(dialog) = query.saved_dialog_mut()
        && let Some(typed) = dialog.name_mut()
    {
        name.clone_into(typed);
    }
    query.submit_saved_dialog();
    app.save_settings();
}

fn names(queries: &SavedQueries) -> Vec<&str> {
    queries
        .iter()
        .map(p4kx_settings::SavedQuery::name)
        .collect()
}

#[test]
fn two_windows_over_one_file_keep_each_others_saved_queries()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut first = app_over(&file);
    let mut second = app_over(&file);

    save(&mut first, "A", "from A");
    save(&mut second, "B", "from B");
    assert_eq!(names(second.model.query().saved()), ["A", "B"], "merged");
    second.model.query_mut().begin_delete(1);
    second.model.query_mut().submit_saved_dialog();
    second.save_settings();
    first.save_settings();

    let stored = Settings::load(&file)?;
    assert_eq!(names(&stored.queries), ["A"], "B saved and deleted, A kept");
    assert_eq!(
        names(first.model.query().saved()),
        ["A"],
        "the window without new edits keeps its list"
    );
    assert!(first.model.query().saved_edits().is_empty(), "stored");
    Ok(())
}

#[test]
fn a_window_without_edits_never_writes_the_saved_queries() -> Result<(), Box<dyn std::error::Error>>
{
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    let mut settings = Settings::default();
    settings.queries.save("Theirs", "from A")?;
    let mut idle = app_over(&file);
    settings.save(&file)?;

    idle.save_settings();

    assert_eq!(names(&Settings::load(&file)?.queries), ["Theirs"], "kept");
    Ok(())
}

#[test]
fn a_file_that_cannot_be_read_is_never_written_over() -> Result<(), Box<dyn std::error::Error>> {
    let folder = tempfile::tempdir()?;
    let file = folder.path().join("settings.toml");
    std::fs::write(&file, "queries = [ broken")?;
    let mut app = app_over(&file);

    save(&mut app, "A", "from A");

    assert_eq!(
        std::fs::read_to_string(&file)?,
        "queries = [ broken",
        "untouched"
    );
    assert_eq!(app.model.query().saved_edits().len(), 1, "kept for later");
    Ok(())
}
