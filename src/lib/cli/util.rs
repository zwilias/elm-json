use super::{ErrorKind, Result};
use crate::{
    package::{self, retriever::Retriever},
    project::{Application, Package, Project},
    semver,
};
use clap::ArgMatches;
use dialoguer::Confirmation;
use failure::ResultExt;
use serde::ser::Serialize;
use slog::Logger;
use std::{
    collections::HashSet,
    convert,
    fs::File,
    io::{BufReader, BufWriter},
};

pub fn confirm(prompt: &str, matches: &ArgMatches) -> Result<bool> {
    if matches.is_present("yes") {
        return Ok(true);
    }
    Confirmation::new()
        .with_text(prompt)
        .interact()
        .context(ErrorKind::Unknown)
        .map_err(convert::Into::into)
}

pub fn with_elm_json<A, P>(
    matches: &ArgMatches,
    logger: &Logger,
    run_app: A,
    run_pkg: P,
) -> Result<()>
where
    A: FnOnce(&ArgMatches, &Logger, Application) -> Result<()>,
    P: FnOnce(&ArgMatches, &Logger, Package) -> Result<()>,
{
    match self::read_elm_json(&matches)? {
        Project::Application(app) => run_app(&matches, &logger, app),
        Project::Package(pkg) => run_pkg(&matches, &logger, pkg),
    }
}

fn read_elm_json(matches: &ArgMatches) -> Result<Project> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path).context(ErrorKind::MissingElmJson)?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader).context(ErrorKind::InvalidElmJson)?;
    Ok(info)
}

pub fn write_elm_json(project: &Project, matches: &ArgMatches) -> Result<()> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::create(path).context(ErrorKind::UnwritableElmJson)?;
    let writer = BufWriter::new(file);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
    project
        .serialize(&mut serializer)
        .context(ErrorKind::Unknown)?;
    Ok(())
}

pub fn add_extra_deps(matches: &ArgMatches, retriever: &mut Retriever) -> HashSet<package::Name> {
    let mut extras = HashSet::new();

    for dep in &matches.values_of_lossy("extra").unwrap_or_else(Vec::new) {
        let parts: Vec<&str> = dep.split('@').collect();
        match parts.as_slice() {
            [name] => {
                let name: package::Name = name.parse().expect("Invalid name parameter");
                retriever.add_dep(name.clone(), &None);
                extras.insert(name);
            }
            [name, version] => {
                let version: semver::Version = version.parse().expect("Invalid version specifier");
                let name: package::Name = name.parse().expect("Invalid name parameter");
                retriever.add_dep(name.clone(), &Some(version));
                extras.insert(name);
            }
            _ => unreachable!(),
        }
    }
    extras
}

pub fn valid_package_name(name: String) -> std::result::Result<(), String> {
    let name: std::result::Result<package::Name, _> = name.parse();
    name.map(|_| ()).map_err(|e| e.to_string())
}

pub fn valid_version(version: String) -> std::result::Result<(), String> {
    let version: std::result::Result<semver::Version, _> = version.parse();
    version.map(|_| ()).map_err(|e| e.to_string())
}

pub fn valid_package(pkg: String) -> std::result::Result<(), String> {
    let parts: Vec<&str> = pkg.split('@').collect();
    match parts.as_slice() {
        [name] => valid_package_name(name.to_string()),
        [name, version] => {
            valid_package_name(name.to_string()).and_then(|_| valid_version(version.to_string()))
        }
        _ => unreachable!(),
    }
}

pub fn format_header(x: &str) -> String {
    format!("-- {} {}", x, "-".repeat(80 - 4 - x.len()))
}
