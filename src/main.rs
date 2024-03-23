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
    man.generate_to(Path::new("."))?;

    // let mut buffer: Vec<u8> = Default::default();
    // man.render(&mut buffer)?;

    // std::fs::write(out_dir.join("mybin.1"), buffer)?;

    Ok(())
}
