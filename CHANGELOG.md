# Changelog

## [Unreleased](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.1.0...main)


### Features


## [0.2.0](https://github.com/dmorawetz/cargo-rpmstatus/compare/v0.1.0...main)


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
