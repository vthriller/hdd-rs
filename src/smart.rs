// error[E0468]: an `extern crate` loading macros must be at the crate root
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;
extern crate regex;
extern crate byteorder;

pub mod ata;
pub mod scsi;

#[cfg(target_os = "linux")]
mod linux_scsi;
#[cfg(target_os = "linux")]
mod linux_ata;
#[cfg(target_os = "freebsd")]
mod freebsd_ata;
#[cfg(target_os = "freebsd")]
mod freebsd_scsi;

pub mod data;
pub mod drivedb;
