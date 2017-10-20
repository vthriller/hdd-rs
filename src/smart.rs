// error[E0468]: an `extern crate` loading macros must be at the crate root
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;
extern crate regex;

// TODO #[cfg(target_os = "linux")]
pub mod scsi;
// TODO #[cfg(target_os = "linux")]
pub mod ata;
#[cfg(target_os = "freebsd")]
pub mod freebsd_ata;

pub mod data;
pub mod drivedb;
