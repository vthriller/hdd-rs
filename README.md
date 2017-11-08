# `hdd`: instruments for querying ATA and SCSI disks

[Documentation](https://docs.rs/hdd/).

This is [work in progress](#to-do).

## Why?

Mainly because I was disappointed in all the available options for putting SMART and SCSI log info into various monitoring systems.

* Scripts that parse `smartctl` output are just ugly, error prone hacks that still make me cringe.
* To add support for different, programming-friendly output formats into `smartctl` (e.g. JSON), one basically needs to rewrite a lot of ad-hoc `printf`s scattered all over the source files, and it's not much easier if you decide to drop the idea of implementing some command-line switch in favour of simply changing the output format altogether. (Things are only getting more complex with `smartd`.)
* `libatasmart` (and tools that it powers) can only work with ATA devices, and only on Linux, and expecting more from that library is simply naïve.

## How?

Build sample tool:

```sh
git clone https://github.com/vthriller/hdd-rs.git
cd smart-rs/cli
cargo build --release
sudo ./target/release/hdd /dev/sda attrs --json
```

Put this into your `Cargo.toml` to start using this crate:
```toml
[dependencies]
hdd = "0.8"
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
* `rg 'TODO|FIXME|XXX|((?i)WTF)|unimplemented!|unwrap|expect' src {cli,sample-scsi}/src build.rs`
* Feature parity with [insert your favourite package name here].
* Support for RAID weirdos (LSI, Adaptec, Areca, you name it) and USB bridges.
* Debugging options (think `smartctl -r ataioctl,2` or `skdump`) for CLI tool.
* More devices (smartmontools can query NVMe devices).
* More platforms (Windows, macOS, \*BSD, Redox…).
* Readable masks (`0b0110_0000`).

## License

This crate is distributed under the terms of Mozilla Public License version 2.0.
