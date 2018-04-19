mod info;
mod health;
mod attrs;
mod list;

use std::collections::HashMap;

lazy_static! {
	pub static ref SUBCOMMANDS: HashMap<&'static str, &'static ::Subcommand> = {
		let mut m: HashMap<&'static str, &'static ::Subcommand> = HashMap::new();
		m.insert("health", &health::Health {});
		m.insert("list",   &list::List {});
		m.insert("info",   &info::Info {});
		m.insert("attrs",  &attrs::Attrs {});
		m
	};
}
