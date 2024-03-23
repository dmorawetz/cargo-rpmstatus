use crate::db::{self, Connection, PkgStatus};
use crate::errors::*;
use crate::graph::Graph;
use cargo_metadata::{Package, PackageId, Source};
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use semver::Version;
use std::path::PathBuf;
use std::thread;

const QUERY_THREADS: usize = 24;

#[derive(Debug, Clone)]
pub struct Pkg {
    pub id: PackageId,
    pub name: String,
    pub version: Version,
    pub source: Option<Source>,
    pub manifest_path: PathBuf,
    pub license: Option<String>,
    pub repository: Option<String>,

    pub rpminfo: Option<RpmInfo>,
}

pub enum PackagingProgress {
    Available,
    NeedsUpdate,
    Missing,
}

use std::fmt;

impl fmt::Display for PackagingProgress {
    //! Generate icons to display the packaging progress.
    //! They should all take the same width when printed in a terminal
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let icon = match self {
            PackagingProgress::Available => "  ",
            PackagingProgress::NeedsUpdate => "⌛",
            PackagingProgress::Missing => "🔴",
        };
        write!(f, "{}", icon)
    }
}

impl Pkg {
    pub fn new(pkg: Package) -> Pkg {
        Pkg {
            id: pkg.id,
            name: pkg.name,
            version: pkg.version,
            source: pkg.source,
            manifest_path: pkg.manifest_path.into(),
            license: pkg.license,
            repository: pkg.repository,

            rpminfo: None,
        }
    }

    pub fn in_fedora(&self) -> bool {
        if let Some(rpm) = &self.rpminfo {
            rpm.in_rawhide
        } else {
            false
        }
    }

    pub fn show_dependencies(&self) -> bool {
        if !self.in_fedora() {
            return true;
        }

        if let Some(rpm) = &self.rpminfo {
            !rpm.exact_match && (rpm.outdated || !rpm.compatible)
        } else {
            true
        }
    }

    pub fn packaging_status(&self) -> PackagingProgress {
        if let Some(rpm) = &self.rpminfo {
            if rpm.in_rawhide {
                if rpm.compatible {
                    // Available at an older yet compatible version
                    PackagingProgress::Available
                } else if rpm.outdated {
                    PackagingProgress::NeedsUpdate
                } else {
                    PackagingProgress::Available
                }
            } else if rpm.outdated {
                PackagingProgress::NeedsUpdate
            } else {
                PackagingProgress::Missing
            }
        } else {
            PackagingProgress::Missing
        }
    }
}

#[derive(Debug, Clone)]
pub struct RpmInfo {
    pub in_rawhide: bool,
    pub outdated: bool,
    pub compatible: bool,
    pub exact_match: bool,
    pub version: String,
}

fn run_task(db: &mut Connection, pkg: Pkg) -> Result<RpmInfo> {
    let mut rpm = RpmInfo {
        in_rawhide: false,
        outdated: false,
        compatible: false,
        exact_match: false,
        version: String::new(),
    };

    let info = db.search(&pkg.name, &pkg.version)?;
    if info.status != PkgStatus::NotFound {
        rpm.in_rawhide = true;
        rpm.version = info.version;
    }

    match info.status {
        PkgStatus::Outdated => rpm.outdated = true,
        PkgStatus::Compatible => rpm.compatible = true,
        PkgStatus::Found => rpm.exact_match = true,
        _ => (),
    }

    Ok(rpm)
}

pub fn populate(graph: &mut Graph) -> Result<(), Error> {
    info!("Updating rawhide repo database");
    db::update_rpm_database()?;

    let (task_tx, task_rx) = crossbeam_channel::unbounded();
    let (return_tx, return_rx) = crossbeam_channel::unbounded();

    info!("Creating thread-pool");
    for _ in 0..QUERY_THREADS {
        let task_rx = task_rx.clone();
        let return_tx = return_tx.clone();

        thread::spawn(move || {
            let mut db = match Connection::new() {
                Ok(db) => db,
                Err(err) => {
                    return_tx.send(Err(err)).unwrap();
                    return;
                }
            };

            for (idx, pkg) in task_rx {
                let deb = run_task(&mut db, pkg);
                if return_tx.send(Ok((idx, deb))).is_err() {
                    break;
                }
            }
        });
    }

    info!("Getting node indices");
    let idxs = graph.graph.node_indices().collect::<Vec<_>>();
    let jobs = idxs.len();
    debug!("Found node indices: {}", jobs);

    for idx in idxs {
        if let Some(pkg) = graph.graph.node_weight_mut(idx) {
            debug!("Adding job for {:?}: {:?}", idx, pkg);
            let pkg = pkg.clone();
            task_tx.send((idx, pkg)).unwrap();
        }
    }

    info!("Processing rpm results");

    let pb = ProgressBar::new(jobs as u64)
        .with_style(
            ProgressStyle::default_bar()
                .template("[{pos:.green}/{len:.green}] {prefix:.bold} {wide_bar}")?,
        )
        .with_prefix("Resolving rpm packages");
    pb.tick();

    for result in return_rx.iter().take(jobs) {
        let result = result.context("A worker crashed")?;

        let idx = result.0;
        let rpm = result.1?;

        if let Some(pkg) = graph.graph.node_weight_mut(idx) {
            pkg.rpminfo = Some(rpm);
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    Ok(())
}
