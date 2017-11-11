/*!
This crate allows you to send various commands to storage devices, and to interpret the answers.

## Example

```
use hdd::Device;
use hdd::scsi::SCSIDevice;

let dev = Device::open("/dev/da0").unwrap();
let (sense, data) = dev.scsi_inquiry(vpd, page).unwrap();
```

TODO show how to send hand-crafted commands, or how to use porcelain interfaces.

For more, dive into documentation for the module you're interested in.
*/

#![warn(missing_debug_implementations)]

#[cfg(feature = "serializable")]
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
