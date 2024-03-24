use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

use args::RpmArgs;
use clap::{CommandFactory, Parser};

use crate::args::{Opts, RpmStatus};
use crate::errors::*;

mod args;
mod db;
mod errors;
mod fedora;
mod format;
mod graph;
mod metadata;
mod tree;

fn main() -> Result<(), Error> {
    env_logger::init();

    let args = match RpmStatus::parse().cmd {
        Opts::Tree(args) => args,
        Opts::Man => {
            return generate_manpage();
        }
    };

    info!("Reading metadata");
    let metadata = metadata::get(&args)?;
    info!("Building graph");
    let mut graph = graph::build(&args, metadata)?;
    info!("Populating with packaging data");
    fedora::populate(&mut graph)?;
    info!("Printing graph");
    tree::print(&args, &graph)?;

    Ok(())
}

fn generate_manpage() -> anyhow::Result<()> {
    let cmd = RpmArgs::command();
    let man = clap_mangen::Man::new(cmd);
    let file = File::create(Path::new("cargo-rpmstatus.1"))?;
    let mut writer = BufWriter::new(file);
    man.render_title(&mut writer)?;
    man.render_name_section(&mut writer)?;
    let synopsis = r"
.SH SYNOPSYS
.PP
\f[B]cargo rpmstatus\f[R] \f[B]OPTIONS\f[R]
.PP
\f[B]cargo rpmstatus --all-features\f[R]
.PP
\f[B]cargo rpmstatus -h\f[R]
";
    write!(writer, "{}", synopsis)?;
    man.render_description_section(&mut writer)?;
    man.render_options_section(&mut writer)?;
    let addendum = r"
.SH AUTHORS
.TP
Daniel Morawetz <daniel@morawetz.dev>
Adapted this manpage from the cargo-debstatus package.
.TP
Matthias Geiger <matthias.geiger1024@tutanota.de>
Wrote the manpage for the Debian system.
.SH COPYRIGHT
.PP
Copyright \[co] 2024 Daniel Morawetz
.PP
Permission is granted to copy, distribute and/or modify this document
under the terms of the GNU General Public License, Version 3 or (at your
option) any later version published by the Free Software Foundation.";
    write!(writer, "{}", addendum)?;
    Ok(())
}
