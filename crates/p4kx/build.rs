// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Embeds the P4KX icon into the Windows executable.

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../p4kx-gui/assets/p4kx.ico");
        resource.compile()?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
