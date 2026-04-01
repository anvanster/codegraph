// Copyright 2024-2026 Andrey Vasilevsky <anvanster@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Embedded authorship and project metadata.
//!
//! These constants are compiled into the binary and can be used
//! to display version and authorship information at runtime.

/// The author of codegraph.
pub const AUTHOR: &str = "Andrey Vasilevsky <anvanster@gmail.com>";

/// The project's SPDX license identifier.
pub const LICENSE: &str = "Apache-2.0";

/// The project's source repository URL.
pub const REPOSITORY: &str = "https://github.com/anvanster/codegraph";

/// The crate version, pulled from Cargo.toml at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The crate name, pulled from Cargo.toml at compile time.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Print a formatted summary of project metadata to stdout.
pub fn print_metadata() {
    println!("{} v{}", NAME, VERSION);
    println!("Author: {}", AUTHOR);
    println!("License: {}", LICENSE);
    println!("Repository: {}", REPOSITORY);
}

/// Return a formatted summary of project metadata as a `String`.
pub fn metadata_string() -> String {
    format!(
        "{} v{}\nAuthor: {}\nLicense: {}\nRepository: {}",
        NAME, VERSION, AUTHOR, LICENSE, REPOSITORY
    )
}
