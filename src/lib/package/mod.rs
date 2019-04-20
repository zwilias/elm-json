use crate::semver::{self, Version};
use failure::{bail, format_err, Error};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::str;

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
    dependencies: BTreeMap<String, Range>,
    test_dependencies: BTreeMap<String, Range>,
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Exposed {
    Plain(Vec<String>),
    Structured(BTreeMap<String, Vec<String>>),
}

impl Package {
    pub fn new(name: String, summary: String, license: String) -> Self {
        let mut dependencies = BTreeMap::new();
        dependencies.insert(
            "elm/core".to_string(),
            Range::new(Version::new(1, 0, 0), Version::new(2, 0, 0)),
        );

        Self {
            name,
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

#[derive(Debug, Copy, Clone)]
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
