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

#[macro_use]
extern crate nom;
extern crate regex;
extern crate byteorder;

extern crate libc;

/// Data transfer direction
#[derive(Debug, Clone, Copy)]
pub enum Direction { None, From, To, Both }

pub mod device;
pub use device::*;

#[cfg(target_os = "freebsd")]
extern crate cam;

pub mod ata;
pub mod scsi;

pub mod drivedb;

mod utils;
