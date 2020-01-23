# `hdd`: instruments for querying ATA and SCSI disks

> ### This repo is dormant.
>
> This code still works as of January 2020 (and I even keep using it to monitor pretty significant number of disk drives), but it is practically unmaintained at this point. The reasons are:
>
> - [smartmontools finally gained support for JSON output with its 7.0 release](https://www.smartmontools.org/browser/tags/RELEASE_7_0/smartmontools/NEWS?rev=5031),
> - I have no time for boring things in an unpopular crate, simple as that. This crate already does what it initially was supposed to do (namely, it enables SMART and SCSI log page monitoring on Linux and FreeBSD systems without unreliable parsing scripts); sure it would be nice to support other ATA/SCSI commands, implement drive-specific hacks from smartmontools, or take time to figure out a nicer way to deal with devices that implement `ATA PASSTHROUGH` than current struct/trait mess, but there is practically no demand for any change whatsoever.
>
> Feel free to send patches or file issues though: even if would not turn out to be a quick 5-minute fix, that will at least show what degree of interest in the crate the audience maintains.

[Documentation](https://docs.rs/hdd/).

This is [work in progress](#to-do).

## Why?

Mainly because I was disappointed in all the available options for putting SMART and SCSI log info into various monitoring systems.

* Scripts that parse `smartctl` output (usually with regexes) are slow, ugly, unreliable hacks.
* ~~To add support for different, programming-friendly output format into `smartctl` (e.g. JSON), one basically needs to rewrite a lot of ad-hoc `printf`s scattered all over the source files, and it's not much easier if you decide to drop the idea of implementing some command-line switch in favour of simply changing the output format altogether. (Things are only getting more complex with `smartd`.)~~
* `libatasmart` (and tools that it powers) can only work with ATA devices, and only on Linux, and expecting more from that library is simply naïve.

## How?

### Prerequisites

This crate can be built on Rust >= 1.26.

### Building CLI tool

```sh
git clone https://github.com/vthriller/hdd-rs.git
cd hdd-rs
cargo build --release --features='bin serializable' --bin=hdd
sudo ./target/release/hdd /dev/sda attrs --json
```

([Sorry if that looks complicated.](https://github.com/rust-lang/cargo/issues/1982))

You can build static binary if, say, you want it for remote GNU/Linux system that runs older version of glibc:

* install musl toolchain (e.g. via `rustup target add x86_64-unknown-linux-musl`),
* append `--target x86_64-unknown-linux-musl` to the `cargo build` line.

### Using library in your code

Put this into your `Cargo.toml`:
```toml
[dependencies]
hdd = "0.10"
```

## What's supported?

Platforms and transports:

* Linux: ATA¹, SCSI
* FreeBSD: ATA, SCSI

SCSI/ATA translation is also supported.

¹ Note that in Linux, ATA is only supported through SAT, although SG_IO kindly emulates that for SATA (and, possibly, PATA?) disks for us.

Features:

* TODO

## To Do

* Documentation.
* Tests.
* More tests.
* Even more tests: big-endian systems, old hardware…
* `rg 'TODO|FIXME|XXX|((?i)WTF)|unimplemented!|\b(unwrap|expect)\b' src sample-scsi/src build.rs`
* Feature parity with [insert your favourite package name here].
* Support for RAID weirdos (LSI, Adaptec, Areca, you name it) and USB bridges.
* Debugging options (think `smartctl -r ataioctl,2` or `skdump`) for CLI tool.
* More devices (smartmontools can query NVMe devices).
* More platforms (Windows, macOS, \*BSD, Redox…).

## Acknowledgements

Here goes [obligatory mention of smartmontools contributors](https://svn.code.sf.net/p/smartmontools/code/trunk/smartmontools/AUTHORS) who laid foundations of what this crate currently is.

## License

This crate is distributed under the terms of Mozilla Public License version 2.0.
