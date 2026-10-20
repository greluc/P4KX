//! The window never offers a target inside a Star Citizen installation and warns while the game
//! runs.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn installation(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let channel = dir.join("StarCitizen").join("LIVE");
    fs::create_dir_all(channel.join("Bin64"))?;
    let path = channel.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\good.txt", "good"))
        .write_to(&path)?;
    Ok(path)
}

fn opened(
    language: Language,
    folder: impl FnOnce(&Path, &Path) -> PathBuf,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = installation(dir.path())?;
    let mut model = Model::new(language);
    model.set_extraction_folder(Some(folder(dir.path(), &path)));
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    Ok((dir, harness))
}

fn wait_until(harness: &mut Harness<'_, Model>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(20);
    harness.run_steps(2);
    while !done(harness.state()) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.run_steps(2);
    }
    harness.run_steps(2);
    Ok(())
}

#[test]
fn a_folder_inside_the_installation_cannot_be_started() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, |_, archive| {
        archive.with_file_name("Bin64").join("..").join("export")
    })?;

    harness.get_by_label("Extract all…").click();
    harness.run_steps(2);

    harness.get_by_label_contains("lies inside the Star Citizen installation");
    assert!(
        harness
            .get_by_label("Extract")
            .accesskit_node()
            .is_disabled(),
        "Start is not offered"
    );
    Ok(())
}

#[test]
fn the_notice_is_in_german_too() -> TestResult {
    let (_dir, mut harness) = opened(Language::German, |_, archive| {
        archive.with_file_name("export")
    })?;

    harness.get_by_label("Alle extrahieren …").click();
    harness.run_steps(2);

    harness.get_by_label_contains("liegt in der Star-Citizen-Installation");
    Ok(())
}

#[test]
fn a_folder_elsewhere_is_fine() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, |dir, _| dir.join("elsewhere"))?;

    harness.get_by_label("Extract all…").click();
    harness.run_steps(2);

    assert!(
        harness
            .query_by_label_contains("lies inside the Star Citizen installation")
            .is_none(),
        "no notice for a folder outside the installation"
    );
    Ok(())
}

static GAME: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct FakeGame(Child);

impl FakeGame {
    fn start(dir: &Path) -> Result<Self, Box<dyn Error>> {
        let (source, name, args): (PathBuf, &str, &[&str]) = if cfg!(windows) {
            let root = std::env::var_os("SystemRoot").ok_or("no SystemRoot")?;
            (
                PathBuf::from(root).join("System32").join("ping.exe"),
                "StarCitizen.exe",
                &["-n", "60", "127.0.0.1"],
            )
        } else {
            (PathBuf::from("/bin/sleep"), "StarCitizen", &["60"])
        };
        let copy = dir.join(name);
        fs::copy(source, &copy)?;
        let child = Command::new(copy)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self(child))
    }
}

impl Drop for FakeGame {
    fn drop(&mut self) {
        let _ignored = self.0.kill();
        let _ignored = self.0.wait();
    }
}

#[test]
fn warns_while_the_game_runs_and_keeps_the_window_usable() -> TestResult {
    let (dir, mut harness) = opened(Language::BritishEnglish, |dir, _| dir.join("elsewhere"))?;
    let _serial = GAME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _game = FakeGame::start(dir.path())?;

    harness.state_mut().watch_game(true);
    wait_until(&mut harness, Model::game_running)?;

    harness.get_by_label_contains("Star Citizen is running");
    harness.get_by_label("Extract all…");
    assert!(
        harness.state().browser().is_some(),
        "the Archive stays open and readable"
    );
    Ok(())
}

#[test]
fn stays_quiet_while_the_game_is_closed() -> TestResult {
    let _serial = GAME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if p4kx_extraction::game_is_running() {
        return Ok(());
    }
    let (_dir, mut harness) = opened(Language::BritishEnglish, |dir, _| dir.join("elsewhere"))?;

    harness.state_mut().watch_game(true);
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        harness.run_steps(2);
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        harness
            .query_by_label_contains("Star Citizen is running")
            .is_none(),
        "no warning without the game"
    );
    Ok(())
}
