use anyhow::Result;
use clap::ArgMatches;
use std::io;

pub fn run(matches: &ArgMatches) -> Result<()> {
    let shell = matches.value_of("SHELL").unwrap();
    super::build().gen_completions_to("elm-json", shell.parse().unwrap(), &mut io::stdout());
    Ok(())
}
