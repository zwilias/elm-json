use clap::ArgMatches;
use failure::Error;
use std::io;

pub fn run(matches: &ArgMatches) -> Result<(), Error> {
    let shell = matches.value_of("SHELL").unwrap();
    super::build().gen_completions_to("elm-json", shell.parse().unwrap(), &mut io::stdout());
    Ok(())
}
