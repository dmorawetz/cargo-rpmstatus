use crate::errors::*;
use sqlite::Connection as SqliteCon;
use sqlite::State;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::env::consts::ARCH;
use std::fs;
use std::fs::File;

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

pub fn update_rpm_database() -> Result<()> {
    let cache_dir = dirs::cache_dir()
            .context("cache directory not found")?
            .join("cargo-rpmstatus");

    debug!("Creating cache dir at {}", &cache_dir.display());
    fs::create_dir_all(&cache_dir)?;

    let path = cache_dir.join("repomd.xml");

    let exists = path.try_exists()?;
    if !exists {
        debug!("repomd.xml did not exist, downloading now ...");
        let url = format!("{}/rawhide/latest/{}/repodata/repomd.xml", KOJI_REPO, ARCH);
        let response =  ureq::get(&url).call().context("could not download repomd.xml")?;
        if response.content_type() != "text/xml" {
            debug!("content type {}", response.content_type());
            bail!("invalid reponse for repomd");
        }
        let mut file = File::create(&path)?;
        std::io::copy(&mut response.into_reader(), &mut file)?;
        file.sync_all()?;
    }

    
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
    pub fn new() -> Result<Connection, Error> {
        debug!("Connecting to database");
        let sock = sqlite::open("/tmp/koji-primary.sqlite")?;
        debug!("Got database connection");

        Ok(Connection { sock })
    }

    pub fn search(&mut self, package: &str, version: &Version) -> Result<PkgInfo, Error> {

        // config.shell().status("Querying", format!("sid: {}", package))?;
        info!("Querying: {}", package);
        let info = self.search_generic(
            "SELECT version FROM packages WHERE name LIKE ?;",
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
        let package = package.replace('_', "-");
        let semver_version = if version.major == 0 {
            if version.minor == 0 {
                format!("{}.{}.{}", version.major, version.minor, version.patch)
            } else {
                format!("{}.{}", version.major, version.minor)
            }
        } else {
            format!("{}", version.major)
        };
        let mut statement = self.sock.prepare(query).unwrap();
        statement.bind((1, format!("rust-{package}%").as_str()))?;


        let version = version.to_string();
        let version = VersionReq::parse(&version)?;
        let semver_version = VersionReq::parse(&semver_version)?;
        while let Ok(State::Row) = statement.next() {
            let rpm_version = statement.read::<String, _>("version").unwrap();

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
    use crate::db::{is_compatible, Connection, PkgStatus};
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
    #[ignore]
    fn check_version_reqs() {
        let mut db = Connection::new().unwrap();
        // Debian bullseye has rust-serde v1.0.106 and shouldn't be updated anymore
        let query =
            "SELECT version::text FROM sources WHERE source in ($1, $2) AND release='bullseye';";
        let info = db
            .search_generic(query, "serde", &Version::parse("1.0.100").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Found);
        assert_eq!(info.version, "1.0.106");
        let info = db
            .search_generic(query, "serde", &Version::parse("1.0.150").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Compatible);
        let info = db
            .search_generic(query, "serde", &Version::parse("2.0.0").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Outdated);
        let info = db
            .search_generic(query, "notacrate", &Version::parse("1.0.0").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::NotFound);
    }

    #[test]
    #[ignore]
    fn check_zerover_version_reqs() {
        let mut db = Connection::new().unwrap();
        // Debian bookworm has rust-zoxide v0.4.3 and shouldn't be updated anymore
        let query =
            "SELECT version::text FROM sources WHERE source in ($1, $2) AND release='bookworm';";
        let info = db
            .search_generic(query, "zoxide", &Version::parse("0.4.1").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Found);
        assert_eq!(info.version, "0.4.3");
        let info = db
            .search_generic(query, "zoxide", &Version::parse("0.4.5").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Compatible);
        let info = db
            .search_generic(query, "zoxide", &Version::parse("0.5.0").unwrap())
            .unwrap();
        assert_eq!(info.status, PkgStatus::Outdated);
    }
}
