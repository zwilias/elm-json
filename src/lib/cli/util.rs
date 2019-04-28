use crate::{
    package::{self, retriever::Retriever},
    project::{Application, Package, Project},
    semver,
};
use clap::ArgMatches;
use colored::Colorize;
use failure::{format_err, Error};
use serde::ser::Serialize;
use slog::Logger;
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufReader, BufWriter},
};

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

pub fn show_diff<K, T>(title: &str, left: &BTreeMap<K, T>, right: &BTreeMap<K, T>)
where
    T: Eq + std::fmt::Display + Sized + Copy,
    K: std::fmt::Display + Ord + Clone,
{
    let it = Diff::new(&left, &right);
    if !it.is_empty() {
        println!(
            "I want to make some changes to your {}{}dependencies\n",
            title.bold(),
            if title.is_empty() { "" } else { " " }
        );
        it.print();
        println!();
    }
}

impl<K, T> Diff<K, T>
where
    T: Sized + Eq + Copy + std::fmt::Display,
    K: std::fmt::Display + Ord + Clone,
{
    pub fn new(left: &BTreeMap<K, T>, right: &BTreeMap<K, T>) -> Diff<K, T> {
        let mut only_left = Vec::new();
        let mut only_right = Vec::new();
        let mut changed = Vec::new();

        let mut iter_left = left.iter();
        let mut iter_right = right.iter();

        let mut left = iter_left.next();
        let mut right = iter_right.next();

        while let (Some((left_name, left_version)), Some((right_name, right_version))) =
            (left, right)
        {
            if left_name == right_name {
                if left_version != right_version {
                    changed.push((left_name.clone(), *left_version, *right_version))
                }

                left = iter_left.next();
                right = iter_right.next();
                continue;
            }

            if left_name < right_name {
                only_left.push((left_name.clone(), *left_version));
                left = iter_left.next();
                continue;
            }

            if left_name > right_name {
                only_right.push((right_name.clone(), *right_version));
                right = iter_right.next();
                continue;
            }
        }

        while let Some((name, version)) = left {
            only_left.push((name.clone(), *version));
            left = iter_left.next();
        }

        while let Some((name, version)) = right {
            only_right.push((name.clone(), *version));
            right = iter_right.next();
        }

        Diff {
            only_left,
            only_right,
            changed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.only_left.is_empty() && self.only_right.is_empty() && self.changed.is_empty()
    }

    pub fn print(&self) {
        for (k, v) in &self.only_left {
            println!("- {} {} {}", "[DEL]".yellow(), k, v);
        }

        for (k, o, n) in &self.changed {
            println!("- {} {} {} -> {}", "[CHG]".blue(), k, o, n);
        }

        for (k, v) in &self.only_right {
            println!("- {} {} {}", "[ADD]".green(), k, v);
        }
    }
}

pub struct Diff<K, T>
where
    K: Ord + std::fmt::Display + Clone,
    T: Eq + Sized + Copy + std::fmt::Display,
{
    only_left: Vec<(K, T)>,
    only_right: Vec<(K, T)>,
    changed: Vec<(K, T, T)>,
}
