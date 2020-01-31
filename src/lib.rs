/*!
This crate allows you to send various commands to storage devices, and to interpret the answers.

## Example

```
use hdd::Device;
use hdd::scsi::{SCSIDevice, SCSICommon};

let dev = Device::open("/dev/da0").unwrap();
let (sense, data) = dev.scsi_inquiry(vpd, page).unwrap();
```

TODO show how to send hand-crafted commands, or how to use porcelain interfaces.

For more, dive into documentation for the module you're interested in.
*/

#![warn(
	missing_debug_implementations,
	// TODO
	//missing_docs,
	// XXX how to limit this to C-like enums? I'd like to #[derive(Copy)] them
	// see also https://github.com/rust-lang-nursery/rust-clippy/issues/2222
	//missing_copy_implementations,
	trivial_casts,
	trivial_numeric_casts,
	// XXX this crate is all about unsafe code, but we should probably limit that to certain modules
	//unsafe_code,
	unstable_features,
	unused_import_braces,
	unused_qualifications,
)]

/* XXX
This lint is here mainly to prevent trait gate method hack from becoming recursive. E.g.:

```
impl Foo<Bar> {
	pub fn foo() {}
}

impl Foo<Baz> {
	pub fn foo() {}
}

impl<T> Whatever for Foo<T> {
	fn foo() { Self::foo() }
}
```

If `Foo<X>::foo()` is gone for some reason (usually during refactoring), `Whatever::foo()` will start calling itself, which is definitely not what we want, so this should be a hard error.
*/
#![deny(unconditional_recursion)]

#[cfg(feature = "serializable")]
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate log;

#[cfg(feature = "drivedb-parser")]
#[macro_use]
extern crate nom;
#[cfg(feature = "drivedb-parser")]
extern crate regex;
extern crate byteorder;

extern crate libc;

/// Data transfer direction
/*
re: Direction::Both:
- freebsd: as of SVN rev 342456 (2018-12-25),
  CAM_DIR_BOTH doesn't seem to be used neither in the kernel nor in the userspace tools like camcontrol,
  except for occasional EINVAL
  (https://github.com/freebsd/freebsd/search?q=CAM_DIR_BOTH)
- linux:
  > The value SG_DXFER_TO_FROM_DEV is only relevant to indirect IO (otherwise it is treated like SG_DXFER_FROM_DEV).
  ~ http://www.tldp.org/HOWTO/SCSI-Generic-HOWTO/x166.html
- 3rd party utils (e.g. smartmontools, sdparm, sg3_utils, libatasmart) have no use for CAM_DIR_BOTH or SG_DXFER_TO_FROM_DEV
*/
#[derive(Debug)]
pub enum Direction<'a> {
	None,
	// `From(usize)` makes functions like do_cmd() unconvenient, as they're required to return Option depending on whether data was requested (`Some(data)`) or not (`None`).
	// This results in unnecessary and potentially dangerous unwrapping, or unnecessary and a tad too verbose checks, copied and scattered all over the code.
	// Pre-allocated buffers greatly simplify consumer's code by removing aforementioned checks and unwraps.
	// The reason this is &Vec<> and not &[] is because we need to truncate it after the operation.
	/**
	Request `vec.capacity()` (*not* `len`!) bytes from the device.

	After an operation completion `vec` is going to be of the actual length of the data transfer.
	*/
	From(&'a mut Vec<u8>),
	To(&'a [u8]),
}

pub mod device;
pub use device::*;

#[cfg(target_os = "freebsd")]
mod cam;

pub mod ata;
pub mod scsi;

#[cfg(feature = "drivedb-parser")]
pub mod drivedb;

mod utils;
