use failure::{format_err, Error};
use semver_constraints;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::str;

pub mod retriever;

#[derive(Serialize, Deserialize)]
pub struct Package {
    name: String,
    version: Version,
    dependencies: HashMap<String, Range>,
    #[serde(flatten)]
    other: HashMap<String, Value>,
}

impl Package {
    pub fn dependencies(&self) -> Vec<(String, Range)> {
        self.dependencies
            .iter()
            .map(|(k, &v)| (k.clone(), v.clone()))
            .collect()
    }
}

#[derive(Copy, Clone)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

#[derive(Copy, Clone)]
pub struct Range {
    lower: Version,
    upper: Version,
}

impl str::FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<u64> = s
            .split('.')
            .map(|x| x.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format_err!("{}", e))?;
        match parts.as_slice() {
            [major, minor, patch] => Ok(Version {
                major: *major,
                minor: *minor,
                patch: *patch,
            }),
            _ => Err(format_err!("Invalid version: {}", s)),
        }
    }
}

impl Version {
    fn to_constraint_version(&self) -> semver_constraints::Version {
        semver_constraints::Version::new(self.major, self.minor, self.patch)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl Range {
    pub fn to_constraint(&self) -> semver_constraints::Constraint {
        let range = semver_constraints::Range::new(
            semver_constraints::Interval::Closed(self.lower.to_constraint_version(), false),
            semver_constraints::Interval::Open(self.upper.to_constraint_version(), false),
        )
        .unwrap();
        semver_constraints::Constraint::from(range)
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
