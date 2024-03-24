use crate::args::RpmArgs;
use anyhow::{anyhow, bail, Context, Error, Result};
use cargo_metadata::Metadata;
use flate2::read::GzDecoder;
use log::{debug, info, trace};
use rand::distributions::{Alphanumeric, DistString};
use std::ffi::OsString;
use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{env, fs};
use tar::Archive;

pub fn get(args: &RpmArgs) -> Result<Metadata> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));

    let mut command = Command::new(cargo);
    command.arg("metadata").arg("--format-version").arg("1");

    if args.quiet {
        command.arg("-q");
    }

    if let Some(features) = &args.features {
        command.arg("--features").arg(features);
    }
    if args.all_features {
        command.arg("--all-features");
    }
    if args.no_default_features {
        command.arg("--no-default-features");
    }

    if !args.all_targets {
        command.arg("--filter-platform");
        match &args.target {
            Some(target) => {
                command.arg(target);
            }
            None => {
                let target = default_target()?;
                command.arg(target);
            }
        }
    }

    if let Some(path) = &args.crate_path {
        let extracted_path = extract_crate_cargo_toml(path)?;
        debug!(
            "Using extracted Cargo.toml at {}",
            &extracted_path.display()
        );
        command.arg("--manifest-path").arg(extracted_path);
    } else if let Some(path) = &args.manifest_path {
        command.arg("--manifest-path").arg(path);
    }

    for _ in 0..args.verbose {
        command.arg("-v");
    }

    if let Some(color) = &args.color {
        command.arg("--color").arg(color);
    }

    if args.frozen {
        command.arg("--frozen");
    }
    if args.locked {
        command.arg("--locked");
    }
    if args.offline {
        command.arg("--offline");
    }

    for flag in &args.unstable_flags {
        command.arg("-Z").arg(flag);
    }

    let output = output(&mut command, "cargo metadata")?;

    serde_json::from_str(&output).context("error parsing cargo metadata output")
}

fn default_target() -> Result<String, Error> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let output = output(Command::new(rustc).arg("-Vv"), "rustc")?;

    for line in output.lines() {
        let prefix = "host: ";
        if let Some(text) = line.strip_prefix(prefix) {
            return Ok(text.trim().to_string());
        }
    }

    Err(anyhow!("host missing from rustc output"))
}

fn output(command: &mut Command, job: &str) -> Result<String, Error> {
    let output = command
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("error running {job}"))?;

    if !output.status.success() {
        return Err(anyhow!("{} returned {}", job, output.status));
    }

    String::from_utf8(output.stdout).with_context(|| format!("error parsing {job} output"))
}

fn extract_crate_cargo_toml(crate_path: &PathBuf) -> Result<PathBuf> {
    let tmp_dir = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let tmp_path = env::temp_dir().join(tmp_dir);

    info!(
        "Creating tmp path for Cargo.toml at {}",
        &tmp_path.display()
    );
    fs::create_dir_all(&tmp_path)?;

    let tar_gz = File::open(crate_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    let manifest = archive.entries()?.filter_map(|e| e.ok()).find(|x| {
        trace!("found entry {:?}", x.path());
        x.path().unwrap_or_default().ends_with("Cargo.toml")
    });

    match manifest {
        Some(mut entry) => {
            let written = entry.unpack_in(&tmp_path)?;

            if !written {
                bail!("could not extract manifest from crate file");
            }

            return Ok(tmp_path.join(entry.path()?));
        }
        None => bail!("could not find manifest file in crate"),
    }
}
