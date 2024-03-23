use std::path::PathBuf;
use std::str::FromStr;

use clap::builder::PathBufValueParser;
use clap::{Parser, Subcommand};

#[derive(Parser, Default)]
#[clap(bin_name = "cargo")]
pub struct RpmStatus {
    #[command(subcommand)]
    pub cmd: Opts,
}

#[derive(Subcommand, Debug)]
pub enum Opts {
    #[command(name = "rpmstatus")]
    /// Display a tree visualization of a dependency graph
    Tree(RpmArgs),
    #[command(name = "mangen")]
    /// Generate a man page
    Man,
}

impl Default for Opts {
    fn default() -> Self {
        Opts::Tree(RpmArgs::default())
    }
}

/// cargo-tree for RPM packaging
///
/// cargo-rpmstatus should be run in a rust project folder.
/// It will then print a dependency graph showing dependencies already in
/// Fedora rawhide and dependencies still missing for packaging.
/// Dependencies already in the rawhide repo will show up green; those not
/// in rawhide yet white.
#[derive(Parser, Debug, Default)]
pub struct RpmArgs {
    #[arg(long = "package", short = 'p', value_name = "SPEC")]
    /// Package to be used as the root of the tree
    pub package: Option<String>,
    #[arg(long = "features", value_name = "FEATURES")]
    /// Space-separated list of features to activate
    pub features: Option<String>,
    #[arg(long = "all-features")]
    /// Activate all available features
    pub all_features: bool,
    #[arg(long = "no-default-features")]
    /// Do not activate the `default` feature
    pub no_default_features: bool,
    #[arg(long = "target", value_name = "TARGET")]
    /// Set the target triple
    pub target: Option<String>,
    #[arg(long = "all-targets")]
    /// Return dependencies for all targets. By default only the host target is matched.
    pub all_targets: bool,
    #[arg(long = "no-dev-dependencies")]
    /// Skip dev dependencies.
    pub no_dev_dependencies: bool,
    #[arg(
        long = "manifest-path",
        value_name = "PATH",
        value_parser(PathBufValueParser::new())
    )]
    /// Path to Cargo.toml
    pub manifest_path: Option<PathBuf>,
    #[arg(long = "invert", short = 'i')]
    /// Invert the tree direction
    pub invert: bool,
    #[arg(long = "no-indent")]
    /// Display the dependencies as a list (rather than a tree)
    pub no_indent: bool,
    #[arg(long = "prefix-depth")]
    /// Display the dependencies as a list (rather than a tree), but prefixed with the depth
    pub prefix_depth: bool,
    #[arg(long = "all", short = 'a')]
    /// Don't truncate dependencies that have already been displayed
    pub all: bool,
    #[arg(long = "duplicate", short = 'd')]
    /// Show only dependencies which come in multiple versions (implies -i)
    pub duplicates: bool,
    #[arg(long = "charset", value_name = "CHARSET", default_value = "utf8")]
    /// Character set to use in output: utf8, ascii
    pub charset: Charset,
    #[arg(
        long = "format",
        short = 'f',
        value_name = "FORMAT",
        default_value = "{p}"
    )]
    /// Format string used for printing dependencies
    pub format: String,
    #[arg(long = "verbose", short = 'v', action = clap::ArgAction::Count)]
    /// Use verbose output (-vv very verbose/build.rs output)
    pub verbose: u8,
    #[arg(long = "quiet", short = 'q')]
    /// No output printed to stdout other than the tree
    pub quiet: bool,
    #[arg(long = "color", value_name = "WHEN")]
    /// Coloring: auto, always, never
    pub color: Option<String>,
    #[arg(long = "frozen")]
    /// Require Cargo.lock and cache are up to date
    pub frozen: bool,
    #[arg(long = "locked")]
    /// Require Cargo.lock is up to date
    pub locked: bool,
    #[arg(long = "offline")]
    /// Do not access the network
    pub offline: bool,
    #[arg(short = 'Z', value_name = "FLAG")]
    /// Unstable (nightly-only) flags to Cargo
    pub unstable_flags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub enum Charset {
    #[default]
    Utf8,
    Ascii,
}

impl FromStr for Charset {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Charset, &'static str> {
        match s {
            "utf8" => Ok(Charset::Utf8),
            "ascii" => Ok(Charset::Ascii),
            _ => Err("invalid charset"),
        }
    }
}
