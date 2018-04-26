use super::Entry;
use regex::bytes::Regex;

#[derive(Debug)]
pub struct DriveDB<'a> {
	entries: Vec<&'a Entry>,
}

impl<'a> DriveDB<'a> {
	pub fn new(entries: &'a Vec<Entry>) -> Self {
		let entries = entries.iter()
			// USB ID entries are parsed differently; also, we don't support USB devices yet
			.filter(|e| ! e.model.starts_with("USB:"))
			.collect();
		DriveDB { entries }
	}
	pub fn find(&self, model: &str, firmware: &str) -> Option<&'a Entry> {
		for entry in self.entries.iter() {
			// model and firmware are expected to be ascii strings, no need to try matching unicode characters

			// > [modelregexp] should never be "".
			let re = Regex::new(format!("(?-u)^{}$", entry.model).as_str()).unwrap();
			if !re.is_match(model.as_bytes()) { continue }

			if ! entry.firmware.is_empty() {
				let re = Regex::new(format!("^(?-u){}$", entry.firmware).as_str()).unwrap();
				if !re.is_match(firmware.as_bytes()) { continue }
			}

			return Some(entry);
		}

		None
	}
	/// Returns default entry from the database (if any).
	pub fn get_default_entry(&self) -> Option<&'a Entry> {
		for entry in self.entries.iter() {
			if entry.family == "DEFAULT" {
				return Some(entry)
			}
		}
		return None
	}
}
