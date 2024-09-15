use crate::errors::*;
use bzip2::read::MultiBzDecoder;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use log::error;
use rusqlite::Connection as SqliteCon;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::Seek;
use std::io::SeekFrom;
use std::time::Duration;
use std::time::SystemTime;

const KOJI_REPO: &str = "https://kojipkgs.fedoraproject.org/repos";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum PkgStatus {
    NotFound,
    Outdated,
    Compatible,
    Found,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PkgInfo {
    pub status: PkgStatus,
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct Repomd {
    revision: u64,
    #[serde(default)]
    data: Vec<RepomdData>,
}

#[derive(Debug, Deserialize)]
struct RepomdData {
    #[serde(rename = "@type")]
    data_type: String,
    location: RepomdLocation,
}

#[derive(Debug, Deserialize)]
struct RepomdLocation {
    #[serde(rename = "@href")]
    href: String,
}

pub fn update_rpm_database(rpmrelease: &String) -> Result<()> {
    let pb = ProgressBar::new(3)
        .with_style(
            ProgressStyle::default_bar()
                .template("[{pos:.green}/{len:.green}] {prefix:.bold} / {msg} {wide_bar}")?,
        )
        .with_prefix("Updating DB")
        .with_message("Checking freshness");
    pb.tick();

    let cache_dir = dirs::cache_dir()
        .context("cache directory not found")?
        .join("cargo-rpmstatus")
        .join(rpmrelease);

    debug!("Creating cache dir at {}", &cache_dir.display());
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("could not create cache dir at {}", &cache_dir.display()))?;

    let repomd_path = cache_dir.join("repomd.xml");
    let primary_db_path = cache_dir.join("primary_db.sqlite");

    let exists = repomd_path.try_exists()?;

    if exists {
        let modified = fs::metadata(&repomd_path)
            .context("could not fetch metadata")?
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let duration = modified.elapsed()?;
        let expired = duration >= Duration::new(24 * 60 * 60, 0);

        if !expired {
            info!("RPM database up-to-date");
            pb.finish_and_clear();
            return Ok(());
        }
    }

    pb.inc(1);
    pb.set_message("Updating repomd.xml");
    debug!("repomd.xml did not exist or was older than 24 hours, downloading now ...");
    let release_path_seg = if rpmrelease.starts_with("rawhide") {
        String::from("rawhide")
    } else {
        format!("{}-build", rpmrelease)
    };
    // for now just download the x86_64 db, because rust libs are mostly noarch
    let url = format!(
        "{}/{}/latest/x86_64/repodata/repomd.xml",
        KOJI_REPO, release_path_seg
    );
    let response = ureq::get(&url)
        .call()
        .context("could not download repomd.xml")?;
    if response.content_type() != "text/xml" {
        debug!("content type {}", response.content_type());
        bail!("invalid reponse for repomd");
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&repomd_path)
        .context("could not create or open repomd.xml")?;
    std::io::copy(&mut response.into_reader(), &mut file).context("could not write repomd.xml")?;
    file.sync_all()
        .context("could not sync repomd file to disk")?;

    file.seek(SeekFrom::Start(0))?;

    let reader = BufReader::new(file);
    let repomd: Repomd = quick_xml::de::from_reader(reader).context("could not parse repomd")?;

    let mut primary_db_location = repomd
        .data
        .into_iter()
        .find(|x| x.data_type.eq("primary_db"))
        .map(|x| x.location.href)
        .context("could not find primary db in repo")?;

    primary_db_location = primary_db_location.replace("repodata/", "");
    debug!("Primary DB located at {}", &primary_db_location);

    pb.inc(1);
    pb.set_message("Updating primary_db.sqlite");

    let url = format!(
        "{}/{}/latest/x86_64/repodata/{}",
        KOJI_REPO, release_path_seg, primary_db_location
    );
    let response = ureq::get(&url)
        .call()
        .context("could not download primary db")?;

    let mut file = File::create(&primary_db_path).context("could not create primary_db.sqlite")?;

    let mut decoder = MultiBzDecoder::new(response.into_reader());
    std::io::copy(&mut decoder, &mut file).context("could not write decompressed primary db")?;
    file.sync_all()
        .context("could not sync primary db to disk")?;

    pb.finish_and_clear();

    info!(
        "successfully updated the RPM database for repo {} to revision {}",
        rpmrelease, &repomd.revision
    );

    Ok(())
}

fn is_compatible(rpmversion: &str, crateversion: &VersionReq) -> Result<bool, Error> {
    let rpmversion = rpmversion.replace('~', "-");
    let rpmversion = Version::parse(&rpmversion)?;

    Ok(crateversion.matches(&rpmversion))
}

pub struct Connection {
    sock: SqliteCon,
}

impl Connection {
    pub fn new(rpmrelease: &String) -> Result<Connection, Error> {
        let cache_dir = dirs::cache_dir()
            .context("cache directory not found")?
            .join("cargo-rpmstatus")
            .join(rpmrelease);

        debug!("Connecting to database");
        let sock = rusqlite::Connection::open(cache_dir.join("primary_db.sqlite"))?;
        debug!("Got database connection");

        Ok(Connection { sock })
    }

    pub fn search(&mut self, package: &str, version: &Version) -> Result<PkgInfo, Error> {
        // config.shell().status("Querying", format!("sid: {}", package))?;
        info!("Querying: {}", package);
        let info = self.search_generic(
            "SELECT version FROM packages WHERE name LIKE ?1;",
            package,
            version,
        )?;

        Ok(info)
    }

    pub fn search_generic(
        &mut self,
        query: &str,
        package: &str,
        version: &Version,
    ) -> Result<PkgInfo, Error> {
        let mut info = PkgInfo {
            status: PkgStatus::NotFound,
            version: String::new(),
        };
        let semver_version = if version.major == 0 {
            if version.minor == 0 {
                format!("{}.{}.{}", version.major, version.minor, version.patch)
            } else {
                format!("{}.{}", version.major, version.minor)
            }
        } else {
            format!("{}", version.major)
        };
        let mut statement = self.sock.prepare(query)?;
        let rpm_version_iter = statement
            .query_map([format!("rust-{package}%").as_str()], |row| {
                Ok(row.get::<_, String>(0)?)
            })?;

        let version = version.to_string();
        let version = VersionReq::parse(&version)?;
        let semver_version = VersionReq::parse(&semver_version)?;
        for rpm_version in rpm_version_iter {
            if let Err(err) = rpm_version {
                error!(
                    "There was an error loading the rpm version from the DB: {}",
                    err
                );
                continue;
            }

            let rpm_version = rpm_version.unwrap();

            if is_compatible(rpm_version.as_str(), &version)? {
                info.version = rpm_version;
                info.status = PkgStatus::Found;
                debug!("{package} {:?}", info);
                return Ok(info);
            } else if is_compatible(rpm_version.as_str(), &semver_version)? {
                info.version = rpm_version;
                info.status = PkgStatus::Compatible;
            } else if info.status == PkgStatus::NotFound {
                info.version = rpm_version;
                info.status = PkgStatus::Outdated;
            }
        }

        debug!("{package} {:?}", info);
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{is_compatible, update_rpm_database, Connection, PkgStatus};
    use semver::{Version, VersionReq};

    #[test]
    fn is_compatible_with_tilde() {
        // The - character is not allowed in RPM versions and is therefore replaced by
        // the ~ character when packaging with rust2rpm.
        // cf. https://docs.fedoraproject.org/en-US/packaging-guidelines/Rust/#_package_versioning
        assert!(is_compatible(
            "1.0.0~alpha.9",
            &VersionReq::parse("1.0.0-alpha.9").unwrap()
        )
        .unwrap());
    }

    #[test]
    fn is_compatible_follows_semver() {
        assert!(is_compatible("0.1.1", &VersionReq::parse("0.1.0").unwrap()).unwrap());
        assert!(!is_compatible("0.1.0", &VersionReq::parse("0.1.1").unwrap()).unwrap());
        assert!(is_compatible("1.1.0", &VersionReq::parse("1").unwrap()).unwrap());
    }

    #[test]
    fn online_check_version_reqs() {
        let rpmrelease = String::from("rawhide");
        // Downloads current packages repository, needs network
        update_rpm_database(&rpmrelease).unwrap();

        let mut db = Connection::new(&rpmrelease).unwrap();
        // Fedora rawhide has rust-serde >= v1.0.188 and rust-serde_json >= v1.0.113
        let info = db
            .search("serde", &Version::parse("1.0.100").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Found);
        let info = db
            .search("serde", &Version::parse("2.0.0").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Outdated);
        let info = db
            .search("notacrate", &Version::parse("1.0.0").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::NotFound);

        let info = db
            .search("serde_json", &Version::parse("1.0.113").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Found);
    }
}
