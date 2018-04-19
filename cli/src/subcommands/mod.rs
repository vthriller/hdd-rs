mod info;
mod health;
mod attrs;
mod list;

use std::collections::HashMap;
use clap::{self, App, ArgMatches};
use ::DeviceArgument;

type Arg = clap::Arg<'static, 'static>;

pub fn arg_json() -> Arg {
	Arg::with_name("json")
		.long("json")
		.help("Export data in JSON")
}

pub fn arg_drivedb() -> Arg {
	Arg::with_name("drivedb")
			.short("B") // smartctl-like
			.long("drivedb") // smartctl-like
			.takes_value(true)
			.multiple(true)
			.value_name("[+]FILE")
			/*
			TODO show what default values are; now it's not possible, temporary value [0] is short-living and `.help()` only accepts &str, not String
			[0]	format!("…\ndefault:\n{}\n{}",
					drivedb_default.join("\n"),
					drivedb_additional.iter().map(|i| format!("+{}", i)).collect::<Vec<_>>().join("\n"),
				)
			*/
			.help("paths to drivedb files to look for\nuse 'FILE' for main (system-wide) file, '+FILE' for additional entries\nentries are looked up in every additional file in order of their appearance, then in the first valid main file, stopping at the first match\n(this option and its behavior is, to some extent, consistent with '-B' from smartctl)")
}

pub trait Subcommand: Sync {
	fn subcommand(&self) -> App<'static, 'static>;
	fn run(&self, path: &Option<&str>, dev: &Option<&DeviceArgument>, args: &ArgMatches);
}

lazy_static! {
	pub static ref SUBCOMMANDS: HashMap<&'static str, &'static Subcommand> = {
		let mut m: HashMap<&'static str, &'static Subcommand> = HashMap::new();
		m.insert("health", &health::Health {});
		m.insert("list",   &list::List {});
		m.insert("info",   &info::Info {});
		m.insert("attrs",  &attrs::Attrs {});
		m
	};
}
