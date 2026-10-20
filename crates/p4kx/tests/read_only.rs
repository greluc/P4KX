//! `p4kx` never writes into a Star Citizen installation and warns while the game runs.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use predicates::prelude::{PredicateBooleanExt, predicate};

type TestResult = Result<(), Box<dyn Error>>;

fn installation(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let channel = root.join("StarCitizen").join("LIVE");
    std::fs::create_dir_all(channel.join("Bin64"))?;
    let archive = channel.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\readme.txt", b"hello".to_vec()))
        .write_to(&archive)?;
    Ok(archive)
}

#[test]
fn extract_refuses_a_target_inside_the_installation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = installation(dir.path())?;
    let out = archive
        .parent()
        .ok_or("no parent")?
        .join("Bin64")
        .join("..")
        .join("export");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data/readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Star Citizen installation"));

    assert!(!out.exists(), "nothing is created inside the installation");
    Ok(())
}

#[test]
fn extract_outside_the_installation_still_works() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = installation(dir.path())?;
    let out = dir.path().join("elsewhere");

    p4kx_testkit::p4kx()?
        .arg("extract")
        .arg(&archive)
        .arg("Data/readme.txt")
        .arg("--out")
        .arg(&out)
        .assert()
        .code(0);

    assert_eq!(std::fs::read(out.join("Data/readme.txt"))?, b"hello");
    Ok(())
}

#[test]
fn a_file_output_inside_the_installation_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = installation(dir.path())?;
    let out = archive.with_file_name("localization.csv");

    p4kx_testkit::p4kx()?
        .arg("loc")
        .arg("export")
        .arg(&archive)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Star Citizen installation"));

    assert!(!out.exists(), "no file is created inside the installation");
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
        let folder = dir.join("game");
        std::fs::create_dir_all(&folder)?;
        let copy = folder.join(name);
        std::fs::copy(source, &copy)?;
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
fn warns_while_the_game_runs_and_keeps_reading() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = installation(dir.path())?;
    let _serial = GAME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _game = FakeGame::start(dir.path())?;

    p4kx_testkit::p4kx()?
        .env_remove("P4KX_GAME_CHECK")
        .arg("list")
        .arg(&archive)
        .assert()
        .code(0)
        .stderr(predicate::str::contains(
            "use this tool with the game closed",
        ))
        .stdout(predicate::str::contains("readme.txt"));
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
    let dir = tempfile::tempdir()?;
    let archive = installation(dir.path())?;

    p4kx_testkit::p4kx()?
        .env_remove("P4KX_GAME_CHECK")
        .arg("list")
        .arg(&archive)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("game closed").not());
    Ok(())
}

#[test]
fn the_check_can_be_switched_off() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = installation(dir.path())?;
    let _serial = GAME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _game = FakeGame::start(dir.path())?;

    p4kx_testkit::p4kx()?
        .arg("list")
        .arg(&archive)
        .assert()
        .code(0)
        .stderr(predicate::str::contains("game closed").not());
    Ok(())
}

#[cfg(windows)]
#[test]
fn a_loose_file_command_protects_the_default_library_without_an_archive() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = dir.path().join("character.chf");
    std::fs::write(&chf, p4kx_testkit::TestChf::v8().to_bytes())?;
    let out = Path::new(r"C:\Program Files\Roberts Space Industries\StarCitizen")
        .join("LIVE")
        .join("user")
        .join("client")
        .join("0")
        .join("CustomCharacters")
        .join("character.json");

    p4kx_testkit::p4kx()?
        .args(["chf", "to-json"])
        .arg(&chf)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Star Citizen installation"));

    assert!(!out.exists(), "nothing is written into the installation");
    Ok(())
}
