use super::Kind;
use crate::{
    package::{self, retriever::Retriever},
    project::{Application, Package, Project},
    semver,
};
use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;
use dialoguer::Confirm;
use serde::ser::Serialize;
use slog::Logger;
use std::{
    collections::HashSet,
    convert,
    fs::File,
    io::{BufWriter, Write},
};

pub fn confirm(prompt: &str, matches: &ArgMatches) -> Result<bool> {
    if matches.is_present("yes") {
        return Ok(true);
    }
    Confirm::new()
        .with_prompt(prompt)
        .interact()
        .context(Kind::Unknown)
        .map_err(convert::Into::into)
}

pub fn with_elm_json<A, P>(
    matches: &ArgMatches,
    offline: bool,
    logger: &Logger,
    run_app: A,
    run_pkg: P,
) -> Result<()>
where
    A: FnOnce(&ArgMatches, bool, &Logger, Application) -> Result<()>,
    P: FnOnce(&ArgMatches, bool, &Logger, Package) -> Result<()>,
{
    match self::read_elm_json(matches)? {
        Project::Application(app) => run_app(matches, offline, logger, app),
        Project::Package(pkg) => run_pkg(matches, offline, logger, pkg),
    }
}

fn read_elm_json(matches: &ArgMatches) -> Result<Project> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path).context(Kind::MissingElmJson)?;
    let info: Project = serde_json::from_reader(file).context(Kind::InvalidElmJson)?;
    Ok(info)
}

pub fn write_elm_json(project: &Project, matches: &ArgMatches) -> Result<()> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::create(path).context(Kind::UnwritableElmJson)?;
    let writer = BufWriter::new(file);
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(writer, formatter);
    project.serialize(&mut serializer).context(Kind::Unknown)?;
    let mut writer = serializer.into_inner();
    writer.write(b"\n").context(Kind::UnwritableElmJson)?;
    writer.flush().context(Kind::UnwritableElmJson)?;
    Ok(())
}

pub fn add_extra_deps(matches: &ArgMatches, retriever: &mut Retriever) -> HashSet<package::Name> {
    let mut extras = HashSet::new();

    for dep in &matches.values_of_lossy("extra").unwrap_or_else(Vec::new) {
        let parts: Vec<&str> = dep.split('@').collect();
        match parts.as_slice() {
            [name] => {
                let name: package::Name = name.parse().expect("Invalid name parameter");
                retriever.add_dep(name.clone(), None);
                extras.insert(name);
            }
            [name, version] => {
                let version: semver::Constraint = version
                    .parse::<semver::Version>()
                    .map(semver::Constraint::from)
                    .or_else(|_| lax_version_from_string(version))
                    .expect("Invalid version specifier");
                let name: package::Name = name.parse().expect("Invalid name parameter");
                retriever.add_dep(name.clone(), Some(version));
                extras.insert(name);
            }
            _ => unreachable!(),
        }
    }
    extras
}

fn lax_version_from_string(version: &str) -> std::result::Result<semver::Constraint, String> {
    let parts: Vec<u64> = version
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("{}", e).to_string())?;
    match parts.as_slice() {
        [major] => Ok(semver::Constraint::from(semver::Range::from(
            &semver::Version::new(*major, 0, 0),
            &semver::Strictness::Safe,
        ))),
        _ => Err("Expected a valid lax version spec".into()),
    }
}

pub fn valid_package_name(name: String) -> std::result::Result<(), String> {
    let name: std::result::Result<package::Name, _> = name.parse();
    name.map(|_| ()).map_err(|e| e.to_string())
}

pub fn valid_version(version: String) -> std::result::Result<(), String> {
    let version: std::result::Result<semver::Version, _> = version.parse();
    version.map(|_| ()).map_err(|e| e.to_string())
}

pub fn valid_lax_version(version: String) -> std::result::Result<(), String> {
    let parts: Vec<u64> = version
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("{}", e).to_string())?;
    match parts.as_slice() {
        [_] => Ok(()),
        _ => Err(anyhow!("Invalid lax version: {}", version).to_string()),
    }
}

pub fn valid_package(pkg: String) -> std::result::Result<(), String> {
    let parts: Vec<&str> = pkg.split('@').collect();
    match parts.as_slice() {
        [name] => valid_package_name((*name).to_string()),
        [name, version] => valid_package_name((*name).to_string()).and_then(|_| {
            valid_version((*version).to_string())
                .or_else(|_| valid_lax_version((*version).to_string()))
        }),
        _ => unreachable!(),
    }
}

pub fn format_header(x: &str) -> String {
    format!("-- {} {}", x, "-".repeat(80 - 4 - x.len()))
}
