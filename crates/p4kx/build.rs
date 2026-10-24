// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Embeds the P4KX icon into the Windows executable.

#[cfg(windows)]
const ICON: &str = "../p4kx-gui/assets/p4kx.ico";

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(ICON);
        resource.compile()?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
