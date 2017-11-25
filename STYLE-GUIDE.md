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
* If for some reason you need to reindent large block of code (e.g. introducing `if … {}` block around it), do it in a separate commit. That way you'll keep commit diffs semantically clean.
* Never import `…::Error` directly, always use external error types with at least one qualifier (e.g. match `Err(io::Error(…))` but not `Err(Error(…))`).
