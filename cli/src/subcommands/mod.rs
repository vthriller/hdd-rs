pub(crate) mod info;
pub(crate) mod health;
pub(crate) mod attrs;
pub(crate) mod list;

pub static subcommands: [&::Subcommand; 4] = [
	&health::Health {},
	&list::List {},
	&info::Info {},
	&attrs::Attrs {},
];
