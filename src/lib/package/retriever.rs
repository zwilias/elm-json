use crate::{
    package,
    semver::{Constraint, Range, Version},
    solver::{incompat::Incompatibility, retriever, summary},
};
use bincode;
use failure::{bail, format_err, Error};
use fs2::FileExt;
use reqwest;
use slog::{debug, o, warn, Logger};
use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

pub struct Retriever {
    deps_cache: HashMap<Summary, Vec<Incompatibility<PackageId>>>,
    versions: HashMap<PackageId, Vec<Version>>,
    preferred_versions: HashMap<PackageId, Version>,
    logger: Logger,
    client: reqwest::Client,
    mode: Mode,
}

type Summary = summary::Summary<PackageId>;

pub enum Mode {
    Minimize,
    Maximize,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PackageId {
    Root,
    Elm,
    Pkg(package::Name),
}

impl summary::PackageId for PackageId {
    fn is_root(&self) -> bool {
        self == &PackageId::Root
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PackageId::Root => write!(f, "root"),
            PackageId::Elm => write!(f, "Elm"),
            PackageId::Pkg(name) => write!(f, "{}", name),
        }
    }
}

impl From<package::Name> for PackageId {
    fn from(n: package::Name) -> Self {
        PackageId::Pkg(n)
    }
}

impl Retriever {
    pub fn new(logger: &Logger, elm_version: &Constraint) -> Result<Self, Error> {
        let mut deps_cache = HashMap::new();

        deps_cache.insert(
            Self::root(),
            vec![Incompatibility::from_dep(
                Self::root(),
                (PackageId::Elm, elm_version.complement()),
            )],
        );

        let logger = logger.new(o!("phase" => "retrieve"));
        let client = reqwest::Client::new();

        let mut retriever = Self {
            deps_cache,
            versions: HashMap::new(),
            preferred_versions: HashMap::new(),
            logger,
            client,
            mode: Mode::Maximize,
        };

        retriever.fetch_versions()?;
        Ok(retriever)
    }

    pub fn minimize(&mut self) {
        self.mode = Mode::Minimize;
    }

    pub fn add_deps<'a, I>(&mut self, deps: I)
    where
        I: IntoIterator<Item = &'a (package::Name, Range)>,
    {
        let entry = self.deps_cache.entry(Self::root()).or_insert_with(Vec::new);
        entry.extend(deps.into_iter().map(|(name, range)| {
            let constraint = Constraint::from(range.clone()).complement();
            Incompatibility::from_dep(Self::root(), (name.clone().into(), constraint))
        }));
    }

    pub fn add_dep(&mut self, name: package::Name, version: &Option<Version>) {
        let constraint =
            version.map_or_else(Constraint::empty, |x| Constraint::from(x).complement());
        let deps = self.deps_cache.entry(Self::root()).or_insert_with(Vec::new);
        deps.push(Incompatibility::from_dep(
            Self::root(),
            (name.into(), constraint),
        ));
    }

    fn count_versions(versions_map: &HashMap<package::Name, Vec<Version>>) -> usize {
        let mut count = 0;
        for vs in versions_map.values() {
            count += vs.len();
        }
        count
    }

    fn fetch_versions(&mut self) -> Result<(), Error> {
        let file = Self::cache_file()?;
        file.lock_exclusive()?;

        let mut versions: HashMap<_, _> = self.fetch_cached_versions(&file).unwrap_or_default();
        let count = Self::count_versions(&versions);

        let remote_versions = self.fetch_remote_versions(count).unwrap_or_else(|_| {
            warn!(
                self.logger,
                "Failed to fetch versions from package.elm-lang.org"
            );
            HashMap::new()
        });
        for (pkg, vs) in &remote_versions {
            let entry = versions.entry(pkg.clone()).or_insert_with(Vec::new);
            entry.extend(vs);
        }

        self.save_cached_versions(&file, &versions)?;
        file.unlock()?;

        let mut versions: HashMap<PackageId, Vec<Version>> = versions
            .iter()
            .map(|(k, v)| (k.clone().into(), v.clone()))
            .collect();

        versions.insert(PackageId::Root, vec![Version::new(1, 0, 0)]);
        versions.insert(
            PackageId::Elm,
            vec![
                Version::new(0, 14, 0),
                Version::new(0, 15, 0),
                Version::new(0, 16, 0),
                Version::new(0, 17, 0),
                Version::new(0, 18, 0),
                Version::new(0, 19, 0),
            ],
        );

        self.versions = versions;
        Ok(())
    }

    fn fetch_cached_versions(
        &self,
        cache_file: &File,
    ) -> Result<HashMap<package::Name, Vec<Version>>, Error> {
        let reader = BufReader::new(cache_file);
        let versions: HashMap<package::Name, Vec<Version>> = bincode::deserialize_from(reader)?;
        Ok(versions)
    }

    fn cache_file() -> Result<File, Error> {
        let mut p_path = Self::packages_path()?;
        p_path.push("elm-json");
        fs::create_dir_all(p_path.clone())?;
        p_path.push("versions.dat");

        OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(p_path)
            .map_err(|_| {
                format_err!("I couldn't open or create the cache file where I cache version info!")
            })
    }

    fn save_cached_versions(
        &self,
        cache_file: &File,
        versions: &HashMap<package::Name, Vec<Version>>,
    ) -> Result<(), Error> {
        let writer = BufWriter::new(cache_file);
        bincode::serialize_into(writer, &versions)?;
        Ok(())
    }

    fn fetch_remote_versions(
        &self,
        from: usize,
    ) -> Result<HashMap<package::Name, Vec<Version>>, Error> {
        debug!(self.logger, "Fetching versions since {}", from);
        let url = format!("https://package.elm-lang.org/all-packages/since/{}", from);
        let mut resp = self.client.get(&url).send()?;

        let versions: Vec<String> = resp.json()?;
        let mut res: HashMap<package::Name, Vec<Version>> = HashMap::new();

        for entry in &versions {
            let parts: Vec<_> = entry.split('@').collect();
            match parts.as_slice() {
                [p, v] => {
                    let name: package::Name = p.parse()?;
                    let version: Version = v.parse()?;
                    let entry = res.entry(name).or_insert_with(Vec::new);
                    entry.push(version)
                }
                _ => bail!("Invalid entry: {}", entry),
            }
        }

        Ok(res)
    }

    // pub fn add_preferred_versions(&mut self, versions: HashMap<PackageId, Version>) {
    pub fn add_preferred_versions<T>(&mut self, versions: T)
    where
        T: IntoIterator<Item = (PackageId, Version)>,
    {
        self.preferred_versions.extend(versions);
    }

    fn fetch_deps(&mut self, pkg: &Summary) -> Result<Vec<Incompatibility<PackageId>>, Error> {
        debug!(
            self.logger,
            "Fetching dependencies for {}@{}", pkg.id, pkg.version
        );

        let url = format!(
            "https://package.elm-lang.org/packages/{}/{}/elm.json",
            pkg.id, pkg.version
        );
        let mut resp = self.client.get(&url).send()?;
        let info: package::Package = resp.json()?;
        Ok(self.deps_from_package(&pkg, &info))
    }

    fn read_cached_deps(
        &mut self,
        pkg: &Summary,
    ) -> Result<Vec<Incompatibility<PackageId>>, Error> {
        debug!(
            self.logger,
            "Attempting to read stored deps for {}@{}", pkg.id, pkg.version
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
    ) -> Vec<Incompatibility<PackageId>> {
        let mut deps: Vec<Incompatibility<_>> = info
            .dependencies
            .iter()
            .map(|(name, range)| {
                let constraint = range.to_constraint().complement();
                Incompatibility::from_dep(pkg.clone(), (name.clone().into(), constraint))
            })
            .collect();

        deps.push(Incompatibility::from_dep(
            pkg.clone(),
            (
                PackageId::Elm,
                info.elm_version().to_constraint().complement(),
            ),
        ));

        debug!(self.logger, "Caching incompatibilities {:#?}", deps);

        self.deps_cache.insert(pkg.clone(), deps.clone());
        deps
    }

    fn packages_path() -> Result<PathBuf, Error> {
        env::var("ELM_HOME")
            .map(PathBuf::from)
            .or_else(|_| {
                if cfg!(windows) {
                    dirs::config_dir()
                        .map(|d| {
                            let mut buf = PathBuf::from(&d);
                            buf.push("elm");
                            buf
                        })
                        .ok_or_else(|| format_err!("No config directory found?"))
                } else {
                    dirs::home_dir()
                        .map(|h| {
                            let mut buf = PathBuf::from(&h);
                            buf.push(".elm");
                            buf
                        })
                        .ok_or_else(|| format_err!("No home directory found?"))
                }
            })
            .map_err(|e| format_err!("{}", e))
    }

    fn root() -> Summary {
        summary::Summary::new(PackageId::Root, Version::new(1, 0, 0))
    }
}

impl retriever::Retriever for Retriever {
    type PackageId = self::PackageId;

    fn root(&self) -> Summary {
        Self::root()
    }

    fn incompats(&mut self, pkg: &Summary) -> Result<Vec<Incompatibility<Self::PackageId>>, Error> {
        if pkg.id == PackageId::Elm {
            return Ok(Vec::new());
        }
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
        debug!(
            self.logger,
            "Finding best version for package {} with constraint {}", pkg, con
        );
        if let Some(version) = self.preferred_versions.get(pkg) {
            if con.satisfies(version) {
                Ok(*version)
            } else {
                bail!(
                    "I want to use version {} for {} but it's not allowed by constraint {}",
                    version,
                    pkg,
                    con
                )
            }
        } else if let Some(versions) = self.versions.get(pkg) {
            versions
                .iter()
                .filter(|v| con.satisfies(v))
                .max_by(|x, y| match self.mode {
                    Mode::Minimize => y.cmp(x),
                    Mode::Maximize => x.cmp(y),
                })
                .cloned()
                .ok_or_else(|| format_err!("Failed to find a version for {}", pkg))
        } else {
            bail!("Unknown package {}", pkg)
        }
    }
}
