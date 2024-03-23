use crate::args::Opts;
use crate::errors::*;
use structopt::StructOpt;

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

    let Opts::Tree(args) = Opts::from_args();
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
