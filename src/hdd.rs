#![warn(missing_debug_implementations)]

// error[E0468]: an `extern crate` loading macros must be at the crate root
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;
extern crate regex;
extern crate byteorder;

/// Data transfer direction
#[derive(Debug)]
pub enum Direction { None, From, To, Both }

pub mod device;
pub use device::*;

#[cfg(target_os = "freebsd")]
mod cam;

pub mod ata;
pub mod scsi;

pub mod drivedb;
