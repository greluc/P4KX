use std::io;
use std::path::Path;

/// Writes a `build_manifest.id` into `channel_folder` in the form the launcher writes it.
///
/// # Errors
///
/// Returns the I/O error of writing the file.
pub fn write_build_manifest(
    channel_folder: &Path,
    branch: &str,
    version: &str,
    change_number: &str,
    build_id: &str,
) -> io::Result<()> {
    let manifest = format!(
        r#"{{
    "Data": {{
        "Branch": "{branch}",
        "BuildDateStamp": "Mon Oct 12 2026",
        "BuildId": "{build_id}",
        "BuildTimeStamp": "02:20:01 PM CST",
        "Config": "shipping",
        "Platform": "pc",
        "RequestedP4ChangeNum": "{change_number}",
        "Shelved_Change": "",
        "Tag": "public",
        "Version": "{version}"
    }}
}}"#
    );
    std::fs::write(channel_folder.join("build_manifest.id"), manifest)
}
