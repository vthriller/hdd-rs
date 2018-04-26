/*!
Use this module to match hard drive and SMART values it returns against smartmontools database.

## Example

```
use hdd::drivedb::{Loader, vendor_attribute};

let loader = Loader::new();
// look for version updated with `update-smart-drivedb(8)` first
loader.load("/var/lib/smartmontools/drivedb/drivedb.h")
	.or_else(|| loader.load("/usr/share/smartmontools/drivedb.h"))?;
// `?` is optional though: if nothing can be loaded, loader will still provide dummy db for us

let db = loader.db();

// extra attribute definitions that user might give
let user_attributes = vec!["9,minutes"]
	.into_iter()
	.map(|attr| vendor_attribute::parse(attr).unwrap())
	.collect();

// TODO: issue ATA IDENTIFY DEVICE cmd and parse the answer here
let id = unimplemented!();

let meta = db.render_meta(&id, &user_attributes);

if let Some(warn) = meta.warning {
	println!("WARNING: {}", warn);
}

if let Some(attr) = meta.render_attribute(9) {
	if let Some(name) = attr.name {
		println!("Attribute 9 is named {}", name);
	}
}
```
*/

mod parser;
mod presets;
mod drivedb;
mod loader;
pub mod vendor_attribute;
pub use self::vendor_attribute::Attribute;
pub use self::drivedb::{DriveDB, DriveMeta};
pub use self::loader::{Loader, Error};

use ata::data::id;

fn filter_presets(id: &id::Id, preset: Vec<Attribute>) -> Vec<Attribute> {
	let drivetype = {
		use self::id::RPM::*;
		use self::vendor_attribute::Type::*;
		match id.rpm {
			RPM(_) => Some(HDD),
			NonRotating => Some(SSD),
			Unknown => None,
		}
	};

	#[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
	preset.into_iter().filter(|attr| match (&attr.drivetype, &drivetype) {
		// this attribute is not type-specific
		(&None, _) => true,
		// drive type match
		(&Some(ref a), &Some(ref b)) if a == b => true,
		// drive type does not match
		(&Some(_), &Some(_)) => false,
		// applying drive-type-specific attributes to drives of unknown type makes no sense
		(&Some(_), &None) => false,
	}).collect()
}
