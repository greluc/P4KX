// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Embeds the P4KX icon into the Windows executable.

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/p4kx.ico");
        resource.compile()?;
    }
    Ok(())
}
