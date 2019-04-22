use crate::{
    package::{
        self,
        retriever::{PackageId, Retriever},
    },
    project::Project,
    semver::{self, Version},
    solver,
};
use clap::ArgMatches;
use colored::Colorize;
use failure::{format_err, Error};
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::BufReader,
};

pub fn read_elm_json(matches: &ArgMatches) -> Result<Project, Error> {
    let path = matches.value_of("INPUT").unwrap();
    let file = File::open(path)
        .map_err(|_| format_err!("I could not read an elm.json file from {}!", path))?;
    let reader = BufReader::new(file);
    let info: Project = serde_json::from_reader(reader)?;
    Ok(info)
}

pub fn find_by_name(
    name: &package::Name,
    g: &solver::Graph<solver::Summary<PackageId>>,
) -> Option<Version> {
    for idx in g.node_indices() {
        let item = g[idx].clone();
        match item.id {
            PackageId::Pkg(n) => {
                if &n == name {
                    return Some(g[idx].version);
                }
                continue;
            }
            _ => continue,
        }
    }
    None
}

pub fn add_extra_deps(
    matches: &ArgMatches,
    retriever: &mut Retriever,
) -> Result<HashSet<package::Name>, Error> {
    let mut extras = HashSet::new();

    for dep in matches
        .values_of_lossy("extra")
        .unwrap_or_else(Vec::new)
        .iter()
    {
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

pub fn error_out(msg: &str, e: Error) {
    println!("\n{}", format_header(msg).cyan());
    println!("\n{}", textwrap::fill(&e.to_string(), 80));
    std::process::exit(1)
}

pub fn format_header(x: &str) -> String {
    format!("-- {} {}", x, "-".repeat(80 - 4 - x.len()))
}

pub fn unsupported(description: &str) -> Result<(), Error> {
    println!("\n{}\n", format_header("COMMAND NOT YET IMPLEMENTED").red());
    println!("{}", textwrap::fill(description, 80));
    std::process::exit(1)
}

pub fn show_diff<K, T>(title: &str, left: &BTreeMap<K, T>, right: &BTreeMap<K, T>)
where
    T: Eq + std::fmt::Display + Sized + Copy,
    K: std::fmt::Display + Ord + Clone,
{
    let it = Diff::new(&left, &right);
    if !it.is_empty() {
        println!(
            "I want to make some changes to your {} dependencies\n",
            title.bold()
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
        for (k, v) in self.only_left.iter() {
            println!("- {} {} {}", "[DEL]".yellow(), k, v);
        }

        for (k, o, n) in self.changed.iter() {
            println!("- {} {} {} -> {}", "[CHG]".blue(), k, o, n);
        }

        for (k, v) in self.only_right.iter() {
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
