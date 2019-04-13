use crate::package;
use crate::solver::{incompat::Incompatibility, retriever, summary};
use failure::{bail, format_err, Error};
use reqwest;
use semver_constraints::{Constraint, Version};
use slog::{o, trace, Logger};
use std::collections::HashMap;

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
        versions.insert("root".to_string(), vec![Version::new(0, 0, 0)]);
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
        Ok(deps)
    }

    fn root() -> Summary {
        summary::Summary::new("root".to_string(), Version::new(0, 0, 0))
    }
}

impl retriever::Retriever for Retriever {
    type PackageId = String;

    fn root(&self) -> Summary {
        Self::root()
    }

    fn incompats(&mut self, pkg: &Summary) -> Result<Vec<Incompatibility<Self::PackageId>>, Error> {
        if let Some(deps) = self.deps_cache.get(&pkg) {
            Ok(deps.clone())
        } else {
            self.fetch_deps(&pkg)
        }
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
