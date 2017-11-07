* For trivial things like indentation style, consult `.editorconfig`; see http://editorconfig.org/ for more.
* Put trailing commas after the last items and match arms to keep future diffs cleaner.
* Consider using `use self::Whatever::*` for matches that use more than 4 enum variants (regardless of number of enums), or if enum qualifiers are quite long, to keep things concise; e.g. prefer this:
```rust
let drivetype = {
	use self::id::RPM::*;
	use self::vendor_attribute::Type::*;
	match id.rpm {
		RPM(_) => Some(HDD),
		NonRotating => Some(SSD),
		Unknown => None,
	}
};
```
over:
```rust
let drivetype = match id.rpm {
	id::RPM::RPM(_) => Some(vendor_attribute::Type::HDD),
	id::RPM::NonRotating => Some(vendor_attribute::Type::SSD),
	id::RPM::Unknown => None,
};
```
