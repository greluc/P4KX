use p4kx_testkit::{TestEntry, other_test_key, test_key};
use serde_json::json;

use crate::support::{Install, TestResult};

const SECRET: &[u8] = b"<secret data of the Archive/>";

fn install() -> TestResult<Install> {
    Install::new(vec![
        TestEntry::stored("Data/plain.xml", b"<plain/>".to_vec()),
        TestEntry::deflated("Data/secret.xml", SECRET.to_vec()).encrypted(),
    ])
}

fn read_secret(server: &mut crate::support::Server) -> TestResult<serde_json::Value> {
    server.call("read_entry_text", &json!({ "path": "Data/secret.xml" }))
}

#[test]
fn without_a_key_listing_and_unencrypted_entries_work_and_encrypted_ones_say_so() -> TestResult {
    let install = install()?;
    let mut server = install.serve_without_key(&["--archive", "LIVE"])?;

    let found = server.ok("search_entries", &json!({ "filter": "secret" }))?;
    assert!(found.to_string().contains("Data/secret.xml"), "{found}");
    let plain = server.ok_text("read_entry_text", &json!({ "path": "Data/plain.xml" }))?;
    assert!(plain.contains("<plain/>"), "{plain}");

    let message = server.error("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(message.contains("encrypted"), "{message}");
    assert!(message.contains("P4KX_ARCHIVE_KEY"), "{message}");
    Ok(())
}

#[test]
fn the_key_from_the_environment_the_option_or_the_profile_reads_the_entry() -> TestResult {
    let install = install()?;
    let hex = test_key().to_hex();

    let mut environment = install.serve(&["--archive", "LIVE"])?;
    let text = environment.ok_text("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(text.contains("secret data"), "environment: {text}");

    let mut option = install.serve_without_key(&["--archive", "LIVE", "--key", &hex])?;
    let text = option.ok_text("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(text.contains("secret data"), "option: {text}");

    std::fs::write(install.config().join("archive.key"), format!("{hex}\n"))?;
    let mut profile = install.serve_without_key(&["--archive", "LIVE"])?;
    let text = profile.ok_text("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(text.contains("secret data"), "profile: {text}");
    Ok(())
}

#[test]
fn a_key_stored_while_the_server_runs_applies_to_the_next_call() -> TestResult {
    let install = install()?;
    let mut server = install.serve_without_key(&["--archive", "LIVE"])?;
    let before = read_secret(&mut server)?;
    assert_eq!(before["isError"], true, "before: {before}");

    std::fs::write(
        install.config().join("archive.key"),
        format!("{}\n", test_key().to_hex()),
    )?;

    let text = server.ok_text("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(text.contains("secret data"), "after: {text}");
    Ok(())
}

#[test]
fn a_wrong_key_is_reported_as_wrong() -> TestResult {
    let install = install()?;
    let wrong = other_test_key().to_hex();
    let mut server = install.serve_without_key(&["--archive", "LIVE", "--key", &wrong])?;

    let message = server.error("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;

    assert!(message.contains("does not"), "{message}");
    assert!(
        !message.contains(&wrong),
        "the key is not repeated: {message}"
    );
    Ok(())
}

#[test]
fn a_mistyped_key_option_stops_the_server_before_it_starts() -> TestResult {
    let install = install()?;
    let output = install.command(&["--key", "not-a-key"]).output()?;

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Archive key"), "{stderr}");
    assert!(
        !stderr.contains("not-a-key"),
        "the input is not repeated: {stderr}"
    );
    Ok(())
}

#[test]
fn a_malformed_stored_key_is_reported_instead_of_read_as_no_key() -> TestResult {
    let install = install()?;
    std::fs::write(install.config().join("archive.key"), "not-a-key-0123\n")?;
    let mut server = install.serve_without_key(&["--archive", "LIVE"])?;

    let message = server.error("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;

    assert!(message.contains("malformed"), "{message}");
    assert!(
        !message.contains("not-a-key-0123"),
        "the content is not repeated: {message}"
    );
    Ok(())
}

#[test]
fn a_corrected_stored_key_applies_to_the_next_call() -> TestResult {
    let install = install()?;
    let key_file = install.config().join("archive.key");
    std::fs::write(&key_file, format!("{}\n", other_test_key().to_hex()))?;
    let mut server = install.serve_without_key(&["--archive", "LIVE"])?;
    let wrong = server.error("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(wrong.contains("does not"), "{wrong}");

    std::fs::write(&key_file, format!("{}\n", test_key().to_hex()))?;
    server.call("archive_info", &json!({}))?;

    let text = server.ok_text("read_entry_text", &json!({ "path": "Data/secret.xml" }))?;
    assert!(text.contains("secret data"), "{text}");
    Ok(())
}
