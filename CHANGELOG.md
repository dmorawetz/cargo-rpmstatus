# Changelog

## [Unreleased](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.2.4...main)

## [0.2.4](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.2.3...v0.2.4)

- Update dependencies for easier packaging

## [0.2.3](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.2.2...v0.2.3)

- Loosen version requirements
- Update man page

## [0.2.2](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.2.1...v0.2.2)

- Loosen version requirements
- Drop the bundled sqlite dependency (needs to be installed separately)

## [0.2.1](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.2.0...v0.2.1)

- Clarify licensing

## [0.2.0](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.1.0...v0.2.0)

### Features

- Get dependency packaging info for .crate files (as downloaded with `rust2rpm -s`)
- Specify Fedora RPM release

### Improvements
- Improve the generation of the man page

### Misc
- Downgrade dependencies to use versions availabe in Fedora

## [0.1.0](https://github.com/dmorawetz/cargo-rpmstatus/tree/v0.1.0) (2024-03-23)

Initial version based off of cargo-debstatus

### Features

- Automatic downloading and updating of Fedora rawhide repodata/primary_db.sqlite
- Compatibility checking between rust and RPM versions
