use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Type { HDD, SSD }
#[derive(Debug, Clone)]
pub struct Attribute {
	pub name: Option<String>,
	pub format: String,
	pub drivetype: Option<Type>,
}
pub type Preset = HashMap<u8, Attribute>;

pub fn parse(line: &String) -> Option<Preset> {
	// using clap here would be an overkill
	let mut args = line.split_whitespace().into_iter();
	let mut output = Preset::new();
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
							let format = match desc.next() {
								Some(x) => x,
								None => return None, // too few
							}; // TODO byte order
							let name = desc.next(); // optional
							let drivetype = desc.next().map(|t| match t {
								"HDD" => Some(Type::HDD),
								"SSD" => Some(Type::SSD),
								_ => None,
							}).unwrap_or(None); // optional
							if desc.next() != None { return None } // too many

							output.insert(id, Attribute {
								name: name.map(|s|s.to_string()),
								format: format.to_string(),
								drivetype: drivetype,
							});
						},
						_ => continue, // TODO other options
					}
				},
			},
		}
	}
}
