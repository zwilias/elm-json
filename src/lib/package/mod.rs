use crate::semver::{self, Version};
use failure::{bail, format_err, Error};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::{
    fmt,
    str::{self, FromStr},
};

pub mod retriever;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    name: String,
    summary: String,
    license: String,
    version: Version,
    exposed_modules: Exposed,
    elm_version: Range,
    pub dependencies: BTreeMap<String, Range>,
    pub test_dependencies: BTreeMap<String, Range>,
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
}

#[derive(Debug)]
pub struct Name {
    author: String,
    project: String,
}

impl Name {
    pub fn new(author: &str, project: &str) -> Result<Self, Error> {
        Self::validate_author(author)?;
        Self::validate_project(project)?;

        Ok(Self {
            author: author.to_string(),
            project: project.to_string(),
        })
    }

    fn validate_author(author: &str) -> Result<(), Error> {
        if author.is_empty() {
            bail!(
                "Author name may not be empty. A valid package name looks like \"author/project\"."
            )
        }

        if author.starts_with('-') {
            bail!("Author name may not start with a dash. Please use your github username!")
        }

        if author.ends_with('-') {
            bail!("Author name may not end with a dash. Please user your github username!")
        }

        if author.contains("--") {
            bail!("Author name may not contain a double dash. Please use your github username!")
        }

        if author.len() > 39 {
            bail!(
                "Author name may not be over 39 characters long. Please use your github username!"
            )
        }

        if !author
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            bail!("Author name may only contain ascii alphanumeric characters.")
        }

        Ok(())
    }

    fn validate_project(project: &str) -> Result<(), Error> {
        if project.is_empty() {
            bail!(
                "Project name maybe not be empty. A valid package name looks like \"author/project\"."
            )
        }

        if project.contains("--") {
            bail!("Project name cannot contain a double dash.")
        }

        if project.ends_with('-') {
            bail!("Project name cannot end with a dash.")
        }

        if !project
            .chars()
            .all(|x| x.is_ascii_lowercase() || x.is_digit(10) || x == '-')
        {
            bail!("Project name may only contains lowercase letters, digits and dashes.")
        }

        if !project.chars().nth(0).unwrap().is_ascii_lowercase() {
            bail!("Project name must start with a letter")
        }

        Ok(())
    }
}

impl FromStr for Name {
    type Err = Error;

    fn from_str(package: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = package.split('/').collect();
        match parts.as_slice() {
            [author, project] => Self::new(author, project),
            _ => Err(format_err!(
                "A valid package name look like \"author/project\""
            )),
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.author, self.project)
    }
}

impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Exposed {
    Plain(Vec<String>),
    Structured(BTreeMap<String, Vec<String>>),
}

impl Package {
    pub fn new(name: Name, summary: String, license: String) -> Self {
        let mut dependencies = BTreeMap::new();
        dependencies.insert(
            "elm/core".to_string(),
            Range::new(Version::new(1, 0, 0), Version::new(2, 0, 0)),
        );

        Self {
            name: format!("{}", name),
            summary,
            license,
            exposed_modules: Exposed::Plain(Vec::new()),
            version: Version::new(1, 0, 0),
            dependencies,
            test_dependencies: BTreeMap::new(),
            elm_version: Range::new(Version::new(0, 19, 0), Version::new(0, 20, 0)),
            other: BTreeMap::new(),
        }
    }

    pub fn elm_version(&self) -> Range {
        self.elm_version
    }

    pub fn with_deps(
        &self,
        dependencies: BTreeMap<String, Range>,
        test_dependencies: BTreeMap<String, Range>,
    ) -> Self {
        Self {
            name: self.name.clone(),
            summary: self.summary.clone(),
            license: self.license.clone(),
            exposed_modules: self.exposed_modules.clone(),
            version: self.version,
            dependencies,
            test_dependencies,
            elm_version: self.elm_version,
            other: self.other.clone(),
        }
    }

    pub fn dependencies(&self) -> Vec<(String, semver::Range)> {
        self.dependencies
            .iter()
            .map(|(k, &v)| (k.clone(), v.to_constraint_range()))
            .collect()
    }

    pub fn all_dependencies(&self) -> Result<Vec<(String, semver::Range)>, Error> {
        let mut all_deps: BTreeMap<String, Range> = self.dependencies.clone();

        for (k, v) in self.test_dependencies.iter() {
            if let Some(e) = all_deps.get(k) {
                bail!(
                    "Dependency {}@{} duplicated in test-dependencies as {}",
                    k,
                    e,
                    v
                )
            }

            all_deps.insert(k.clone(), *v);
        }

        Ok(all_deps
            .iter()
            .map(|(k, v)| (k.clone(), v.to_constraint_range()))
            .collect())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Range {
    lower: Version,
    upper: Version,
}

impl Range {
    pub fn new(lower: Version, upper: Version) -> Self {
        Self { lower, upper }
    }

    pub fn to_constraint(&self) -> semver::Constraint {
        self.to_constraint_range().into()
    }

    pub fn to_constraint_range(&self) -> semver::Range {
        semver::Range::new(
            semver::Interval::Closed(self.lower),
            semver::Interval::Open(self.upper),
        )
        .unwrap()
    }
}

impl str::FromStr for Range {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(' ').collect();
        match parts.as_slice() {
            [lower, "<=", "v", "<", upper] => {
                let lower: Version = lower.to_string().parse()?;
                let upper: Version = upper.to_string().parse()?;
                Ok(Range { lower, upper })
            }
            _ => Err(format_err!("Invalid range: {}", s)),
        }
    }
}

impl From<Version> for Range {
    fn from(v: Version) -> Self {
        Self {
            lower: v,
            upper: Version::new(v.major() + 1, 0, 0),
        }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} <= v < {}", self.lower, self.upper)
    }
}

impl Serialize for Range {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Range {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name() {
        assert!("foo/bar".parse::<Name>().is_ok());
        assert!("foo-bar-123/bar".parse::<Name>().is_ok());
        assert!("1/bar".parse::<Name>().is_ok());
        assert!("foo/b-r".parse::<Name>().is_ok());

        assert!("".parse::<Name>().is_err());
        assert!("/".parse::<Name>().is_err());
        assert!("foo/".parse::<Name>().is_err());
        assert!("/bar".parse::<Name>().is_err());
        assert!("\n/bar".parse::<Name>().is_err());
        assert!("-foo/bar".parse::<Name>().is_err());
        assert!("foo-/bar".parse::<Name>().is_err());
        assert!("foo/ba-".parse::<Name>().is_err());
    }
}
