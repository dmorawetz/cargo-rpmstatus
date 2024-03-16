# cargo-rpmstatus

cargo-tree for rpm packaging. Traverse all dependencies of a project, checks
if the dependency is already in fedora rawhide, or if it needs
to be updated.

The codebase is a modified version of [kpcyrd/cargo-debstatus](https://github.com/kpcyrd/cargo-debstatus).

## How to run

```shell
$ cargo install cargo-rpmstatus
$ cargo rpmstatus
```

![screenshot](screenshot.png)

## Known Bugs

- Some indirect optional dependencies are ignored

## License

GPLv3+
