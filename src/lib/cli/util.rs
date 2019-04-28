use crate::{
    package::{self, retriever::Retriever},
    project::{Application, Package, Project},
    semver,
};
use clap::ArgMatches;
use colored::Colorize;
use dialoguer::Confirmation;
use failure::{format_err, Error};
use serde::ser::Serialize;
use slog::Logger;
use std::{
    collections::HashSet,
    fs::File,
    io::{self, BufReader, BufWriter},
};

pub fn confirm(prompt: &str, matches: &ArgMatches) -> Result<bool, io::Error> {
    if matches.is_present("yes") {
        return Ok(true);
    }
    Confirmation::new().with_text(prompt).interact()
}

pub fn with_elm_json<A, P>(
    matches: &ArgMatches,
    logger: &Logger,
    run_app: A,
    run_pkg: P,
) -> Result<(), Error>
where
    A: FnOnce(&ArgMatches, &Logger, &Application) -> Result<(), Error>,
    P: FnOnce(&ArgMatches, &Logger, &Package) -> Result<(), Error>,
{
    match self::read_elm_json(&matches)? {
        Project::Application(app) => run_app(&matches, &logger, &app),
        Project::Package(pkg) => run_pkg(&matches, &logger, &pkg),
    }
}

fn read_elm_json(matches: &ArgMatches) -> Result<Project, Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)
        .map_err(|_| format_err!("I could not read an elm.json file from {}!", path))?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;
    Ok(info)
}

pub fn write_elm_json(project: &Project, matches: &ArgMatches) -> Result<(), Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::create(path).map_err(|_| {
        format_err!(
            "I tried to write to {} but failed to do so. Do you have the required access rights?",
            path
        )
    })?;
    let writer = BufWriter::new(file);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
    project.serialize(&mut serializer)?;
    Ok(())
}

pub fn add_extra_deps(
    matches: &ArgMatches,
    retriever: &mut Retriever,
) -> Result<HashSet<package::Name>, Error> {
    let mut extras = HashSet::new();

    for dep in &matches.values_of_lossy("extra").unwrap_or_else(Vec::new) {
        let parts: Vec<&str> = dep.split('@').collect();
        match parts.as_slice() {
            [name] => {
                let name: package::Name = name.parse()?;
                retriever.add_dep(name.clone(), &None);
                extras.insert(name);
            }
            [name, version] => {
                let version: semver::Version = version.parse()?;
                let name: package::Name = name.parse()?;
                retriever.add_dep(name.clone(), &Some(version));
                extras.insert(name);
            }
            _ => unreachable!(),
        }
    }
    Ok(extras)
}

pub fn error_out(msg: &str, e: &Error) {
    println!("\n{}", format_header(msg).cyan());
    println!("\n{}", textwrap::fill(&e.to_string(), 80));
    std::process::exit(1)
}

pub fn format_header(x: &str) -> String {
    format!("-- {} {}", x, "-".repeat(80 - 4 - x.len()))
}
