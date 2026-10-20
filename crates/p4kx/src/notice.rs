//! The legal notice of `p4kx --help` and `p4kx --version`.

macro_rules! legal_notice {
    () => {
        "Unofficial Star Citizen fan project, not affiliated with the Cloud Imperium group of \
         companies. Ships no game data and reads only your own installation. You are responsible \
         for complying with the Star Citizen EULA and Terms of Service."
    };
}

/// Shown above the usage of `--help`.
pub(crate) const LEGAL_NOTICE: &str = legal_notice!();

/// Shown by `--version`: the version, then the legal notice.
pub(crate) const LONG_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\n", legal_notice!());
