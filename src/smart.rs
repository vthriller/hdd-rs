// error[E0468]: an `extern crate` loading macros must be at the crate root
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;
extern crate regex;
extern crate byteorder;

pub mod ata;
pub mod scsi;

pub mod data;
pub mod drivedb;
