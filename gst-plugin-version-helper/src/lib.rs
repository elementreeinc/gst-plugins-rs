// Copyright (C) 2019 Sajeer Ahamed <ahamedsajeer.15.15@cse.mrt.ac.lk>
// Copyright (C) 2019 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! Extracts release for [GStreamer](https://gstreamer.freedesktop.org) plugin metadata
//!
//! See [`get_info`](fn.get_info.html) for details.
//!
//! This function is supposed to be used as follows in the `build.rs` of a crate that implements a
//! plugin:
//!
//! ```rust,ignore
//! extern crate gst_plugin_version_helper;
//!
//! gst_plugin_version_helper::get_info();
//! ```
//!
//! Inside `lib.rs` of the plugin, the information provided by `get_info` are usable as follows:
//!
//! ```rust,ignore
//! gst_plugin_define!(
//!     the_plugin_name,
//!     env!("CARGO_PKG_DESCRIPTION"),
//!     plugin_init,
//!     concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
//!     "The Plugin's License",
//!     env!("CARGO_PKG_NAME"),
//!     env!("CARGO_PKG_NAME"),
//!     env!("CARGO_PKG_REPOSITORY"),
//!     env!("BUILD_REL_DATE")
//! );
//! ```

mod git;

use std::{env, fs, path};

/// Extracts release for GStreamer plugin metadata
///
/// Release information is first tried to be extracted from a git repository at the same
/// place as the `Cargo.toml`, or one directory up to allow for Cargo workspaces. If no
/// git repository is found, the information is extract from a `release.txt` in the same
/// directory as the `Cargo.toml`.
///
/// - If extracted from a git repository, sets the `COMMIT_ID` environment variable to the short
///   commit id of the latest commit and the `BUILD_REL_DATE` environment variable to the date of the
///   commit.
///
/// - If extracted from a `release.txt`, `COMMIT_ID` will be set to the string `RELEASE` and the
///   `BUILD_REL_DATE` variable will be set to the release date extracted from `release.txt`.
///
/// - If neither is possible, `COMMIT_ID` is set to the string `UNKNOWN` and `BUILD_REL_DATE` to the
///   current date.
///
/// ## `release.txt`
///
/// `release.txt` is only parsed if no git repository was found and is assumed to be in the format
///
/// ```txt
/// version_number
/// release_date
/// ```
///
/// for example
///
/// ```txt
/// 1.16.0
/// 2019-04-19
/// ```
///
/// If the version number from `Cargo.toml` is not equivalent to the one in `release.txt`, this
/// function will panic.
pub fn get_info() {
    let mut commit_id = "UNKNOWN".into();
    let mut commit_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let crate_dir =
        path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let mut repo_dir = crate_dir.clone();

    // First check for a git repository in the manifest directory and if there
    // is none try one directory up in case we're in a Cargo workspace
    let git_info = git::repo_hash(&repo_dir).or_else(move || {
        repo_dir.pop();
        git::repo_hash(&repo_dir)
    });

    let mut release_file = crate_dir.clone();
    release_file.push("release.txt");

    // If there is a git repository, extract the version information from there.
    // Otherwise use a 'release.txt' file as fallback, or report the current
    // date and an unknown revision.
    if let Some((id, date)) = git_info {
        commit_id = id;
        commit_date = date;
    } else if let Ok(release_file) = fs::File::open(release_file) {
        let mut cargo_toml = crate_dir;
        cargo_toml.push("Cargo.toml");

        if let Ok(cargo_toml) = fs::File::open(cargo_toml) {
            commit_date = read_release_date(cargo_toml, release_file);
            commit_id = "RELEASE".into();
        }
    }

    println!("cargo:rustc-env=COMMIT_ID={}", commit_id);
    println!("cargo:rustc-env=BUILD_REL_DATE={}", commit_date);
}

fn read_release_date(mut cargo_toml: fs::File, mut release_file: fs::File) -> String {
    use std::io::Read;

    let mut content = String::new();
    release_file
        .read_to_string(&mut content)
        .expect("Failed to read version file");

    let mut lines = content.lines();
    let version = lines.next().expect("Failed to read version number");
    let date = lines.next().expect("Failed to read release date");

    let mut cargo_toml_content = String::new();
    cargo_toml
        .read_to_string(&mut cargo_toml_content)
        .expect("Failed to read `Cargo.toml`");

    let cargo_toml = cargo_toml_content
        .parse::<toml::Value>()
        .expect("Failed to parse `Cargo.toml`");
    let package = cargo_toml["package"]
        .as_table()
        .expect("Failed to find 'package' section in `Cargo.toml`");
    let cargo_version = package["version"]
        .as_str()
        .expect("Failed to read version from `Cargo.toml`");

    if cargo_version != version {
        panic!(
            "Version from 'version.txt' `{}` different from 'Cargo.toml' `{}`",
            version, cargo_version
        );
    }

    date.into()
}
