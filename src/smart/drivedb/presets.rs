use std::collections::HashMap;

pub type Preset = HashMap<u8, String>;

pub fn parse(line: &String) -> Option<Preset> {
	// using clap here would be an overkill
	let mut args = line.split_whitespace().into_iter();
	let mut output = HashMap::<u8, String>::new();
	loop {
		match args.next() {
			None => return Some(output),
			Some(key) => match args.next() {
				None => return None, // we always expect an argument for the option
				Some(value) => {
					match key {
						"-v" => {
							// parse argument of format 'ID,FORMAT[:BYTEORDER][,NAME]'
							let mut desc = value.split(',');
							// TODO:
							// > If 'N' is specified as ID, the settings for all Attributes are changed.
							let id = match desc.next() {
								Some(x) => match x.parse::<u8>() {
									Ok(x) => x,
									Err(_) => return None, // invalid number
								},
								None => return None, // too few
							};
							let _format = match desc.next() {
								Some(x) => x,
								None => return None, // too few
							}; // TODO
							let name = desc.next(); // optional
							let _type = desc.next(); // optional, either "HDD" or "SSD"; TODO
							if desc.next() != None { return None } // too many

							match name {
								None => continue,
								Some(name) => { output.insert(id, name.to_string()); }
							}
						},
						_ => continue, // TODO other options
					}
				},
			},
		}
	}
}
