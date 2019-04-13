use crate::{
    package,
    solver::{incompat::Incompatibility, retriever, summary},
};
use failure::{bail, format_err, Error};
use reqwest;
use semver_constraints::{Constraint, Version};
use slog::{o, trace, Logger};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

pub struct Retriever {
    deps_cache: HashMap<Summary, Vec<Incompatibility<String>>>,
    versions: HashMap<String, Vec<Version>>,
    logger: Logger,
    client: reqwest::Client,
}

type Summary = summary::Summary<String>;

impl Retriever {
    pub fn new(logger: &Logger, deps: &[(String, package::Range)]) -> Self {
        let mut deps_cache = HashMap::new();

        let deps: Vec<Incompatibility<String>> = deps
            .iter()
            .map(|(name, range)| {
                let constraint = range.to_constraint().complement();
                Incompatibility::from_dep(Self::root(), (name.clone(), constraint))
            })
            .collect();

        deps_cache.insert(Self::root(), deps);

        let logger = logger.new(o!("phase" => "retrieve"));
        let client = reqwest::Client::new();

        Retriever {
            deps_cache,
            versions: HashMap::new(),
            logger,
            client,
        }
    }

    pub fn add_dep(&mut self, name: &String, version: &Option<package::Version>) {
        let constraint =
            version.map_or_else(|| Constraint::empty(), |x| x.to_constraint().complement());
        let deps = self.deps_cache.entry(Self::root()).or_insert(Vec::new());
        deps.push(Incompatibility::from_dep(
            Self::root(),
            (name.clone(), constraint),
        ));
    }

    pub fn fetch_versions(&mut self) -> Result<(), Error> {
        let mut resp = self
            .client
            .get("https://package.elm-lang.org/all-packages")
            .send()?;

        let versions: HashMap<String, Vec<package::Version>> = resp.json()?;
        let mut versions: HashMap<String, Vec<Version>> = versions
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.iter().map(|v| v.to_constraint_version()).collect(),
                )
            })
            .collect();
        versions.insert("root".to_string(), vec![Version::new(1, 0, 0)]);
        self.versions = versions;
        Ok(())
    }

    fn fetch_deps(&mut self, pkg: &Summary) -> Result<Vec<Incompatibility<String>>, Error> {
        trace!(
            self.logger,
            "Fetching dependencies for {}@{}",
            pkg.id,
            pkg.version
        );

        let url = format!(
            "https://package.elm-lang.org/packages/{}/{}/elm.json",
            pkg.id, pkg.version
        );
        let mut resp = self.client.get(&url).send()?;
        let info: package::Package = resp.json()?;
        Ok(self.deps_from_package(&pkg, &info))
    }

    fn read_cached_deps(&mut self, pkg: &Summary) -> Result<Vec<Incompatibility<String>>, Error> {
        trace!(
            self.logger,
            "Attempting to read stored deps for {}@{}",
            pkg.id,
            pkg.version
        );

        let mut p_path = Self::packages_path()?;
        p_path.push(format!(
            "0.19.0/package/{}/{}/elm.json",
            pkg.id, pkg.version
        ));

        let file = File::open(p_path)?;
        let reader = BufReader::new(file);
        let info: package::Package = serde_json::from_reader(reader)?;

        Ok(self.deps_from_package(&pkg, &info))
    }

    fn deps_from_package(
        &mut self,
        pkg: &Summary,
        info: &package::Package,
    ) -> Vec<Incompatibility<String>> {
        let deps: Vec<Incompatibility<String>> = info
            .dependencies
            .iter()
            .map(|(name, range)| {
                let constraint = range.to_constraint().complement();
                Incompatibility::from_dep(pkg.clone(), (name.clone(), constraint))
            })
            .collect();

        trace!(self.logger, "Caching incompatibilities {:#?}", deps);

        self.deps_cache.insert(pkg.clone(), deps.clone());
        deps
    }

    fn packages_path() -> Result<PathBuf, Error> {
        env::var("ELM_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                env::var("HOME").map(|h| {
                    let mut buf = PathBuf::from(&h);
                    buf.push(".elm");
                    buf
                })
            })
            .map_err(|e| format_err!("{}", e))
    }

    fn root() -> Summary {
        summary::Summary::new("root".to_string(), Version::new(1, 0, 0))
    }
}

impl retriever::Retriever for Retriever {
    type PackageId = String;

    fn root(&self) -> Summary {
        Self::root()
    }

    fn incompats(&mut self, pkg: &Summary) -> Result<Vec<Incompatibility<Self::PackageId>>, Error> {
        self.deps_cache
            .get(&pkg)
            .cloned()
            .ok_or(())
            .or_else(|_| self.read_cached_deps(&pkg))
            .or_else(|_| self.fetch_deps(&pkg))
    }

    fn count_versions(&self, pkg: &Self::PackageId) -> usize {
        if let Some(versions) = self.versions.get(pkg) {
            versions.len()
        } else {
            0
        }
    }

    fn best(&mut self, pkg: &Self::PackageId, con: &Constraint) -> Result<Version, Error> {
        trace!(
            self.logger,
            "Finding best version for package {} with constraint {}",
            pkg,
            con
        );
        if let Some(versions) = self.versions.get(pkg) {
            versions
                .iter()
                .filter(|v| con.satisfies(v))
                .max_by(|x, y| x.cmp(y))
                .cloned()
                .ok_or_else(|| format_err!("Failed to find a version for {}", pkg))
        } else {
            bail!("Unknown package {}", pkg)
        }
    }
}
