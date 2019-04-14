use crate::semver::{self, Version};
use failure::{bail, format_err, Error};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::str;

pub mod retriever;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    name: String,
    version: Version,
    dependencies: HashMap<String, Range>,
    test_dependencies: HashMap<String, Range>,
    elm_version: Range,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

impl Package {
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
        let mut all_deps: HashMap<String, Range> = self.dependencies.clone();

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

#[derive(Copy, Clone)]
pub struct Range {
    lower: Version,
    upper: Version,
}

impl Range {
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
