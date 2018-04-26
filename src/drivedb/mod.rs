/*!
Use this module to match hard drive and SMART values it returns against smartmontools database.

## Example

```
use hdd::drivedb;
use hdd::drivedb::vendor_attribute;

// look for version updated with `update-smart-drivedb(8)` first
let drivedb = drivedb::load("/var/lib/smartmontools/drivedb/drivedb.h").or(
	drivedb::load("/usr/share/smartmontools/drivedb.h")
)?;

// extra attribute definitions that user might give
let user_attributes = vec!["9,minutes"]
	.into_iter()
	.map(|attr| vendor_attribute::parse(attr).unwrap())
	.collect();

// TODO: issue ATA IDENTIFY DEVICE cmd and parse the answer here
let id = unimplemented!();

let dbentry = drivedb::match_entry(
	&id,
	&drivedb,
	user_attributes,
);

if let Some(warn) = dbentry.warning {
	println!("WARNING: {}", warn);
}
```
*/

mod parser;
mod presets;
mod drivedb;
mod loader;
pub mod vendor_attribute;
pub use self::vendor_attribute::Attribute;
pub use self::drivedb::DriveDB;
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

/// Drive-related data that cannot be queried from the drive itself (model family, attribute presets etc.)
#[derive(Debug)]
pub struct DriveMeta<'a> {
	/// > Informal string about the model family/series of a device.
	pub family: Option<&'a String>,

	/// > A message that may be displayed for matching drives.
	/// > For example, to inform the user that they may need to apply a firmware patch.
	pub warning: Option<&'a String>,

	/// SMART attribute descriptions
	pub presets: Vec<Attribute>,
}

impl<'a> DriveMeta<'a> {
	pub fn render_attribute(&'a self, id: u8) -> Option<Attribute> {
		vendor_attribute::render(self.presets.to_vec(), id)
	}
}
