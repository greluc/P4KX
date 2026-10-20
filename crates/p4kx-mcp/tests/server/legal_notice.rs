use crate::support::{Install, TestResult};

const STATEMENTS: [&str; 5] = [
    "Unofficial Star Citizen fan project",
    "not affiliated with or endorsed by the Cloud Imperium group of companies",
    "Ships no game data and reads only your own installation",
    "EULA and Terms of Service",
    "must not be redistributed",
];

fn output_of(flag: &str) -> TestResult<String> {
    let install = Install::standard()?;
    let output = install.command(&[flag]).output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn help_states_the_legal_notice() -> TestResult {
    let help = output_of("--help")?;
    for statement in STATEMENTS {
        assert!(help.contains(statement), "{statement}: {help}");
    }
    Ok(())
}

#[test]
fn version_states_the_legal_notice() -> TestResult {
    let version = output_of("--version")?;
    for statement in &STATEMENTS[..4] {
        assert!(version.contains(statement), "{statement}: {version}");
    }
    Ok(())
}
