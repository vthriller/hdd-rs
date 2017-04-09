use super::vendor_attribute;
use super::vendor_attribute::Attribute;

pub fn parse(line: &String) -> Option<Vec<Attribute>> {
	// using clap here would be an overkill
	let mut args = line.split_whitespace().into_iter();
	let mut output = Vec::<Attribute>::new();
	loop {
		match args.next() {
			None => return Some(output),
			Some(key) => match args.next() {
				None => return None, // we always expect an argument for the option
				Some(value) => {
					match key {
						"-v" => { match vendor_attribute::parse(value) {
							Ok(attr) => output.push(attr),
							Err(_) => (), // TODO
						} },
						_ => continue, // TODO other options
					}
				},
			},
		}
	}
}
