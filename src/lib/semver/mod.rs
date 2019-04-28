/*
Copyright (c) 2019 David Cao

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
//! Defines modified syntax for version constraints.
//!
//! ## NIH?
//! The semver crate's `Version` is fine. What's not fine is their `VersionReq.`
//!
//! The reason we're rolling our own instead of using something like the semver crate is that
//! the requirements for elba conflict with what semver provides. The vector-of-predicate
//! approach which semver provides is too flexible, making it harder to validate versions and
//! perform operations on them (check if one range is a subset of another, etc.). The semver crate
//! also provides some unnecessary operations.
//!
//! Instead, this module adds features in some places and removes others for flexibility where it
//! matters for elba.
//!
//! ## Functionality
//! Versions in elba take lots of good ideas from Cargo and Pub (Dart) versioning. We follow
//! Cargo's compatibility rules for 0.* and 0.0.* versions to allow for less-stable packages.
//! Additionally, we also follow Cargo's rules when sigils are omitted.
//! However, we purposely elide star notation since it's unnecessary; `0.* == 0`, `0.0.* == 0.0`.
//! To make parsing easier, `<` or `<=` must always precede `>` or `>=`, like with Pub. Nonsensical
//! requirements like `< 1 > 2` which are valid parses under semver get caught during parsing here.
//! In general, syntax is substantially stricter than in Cargo, and nonsensical constraints are
//! caught immediately when creating the constraint.

// Good ideas: https://pub.dartlang.org/packages/pub_semver

use self::Interval::{Closed, Open, Unbounded};
use failure::{format_err, Error};
use indexmap::{indexset, IndexSet};
use itertools::Itertools;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{cmp, fmt, str::FromStr, string::ToString};

pub enum Strictness {
    Exact,
    Safe,
    Unsafe,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Relation {
    Superset,
    Subset,
    Overlapping,
    Disjoint,
    Equal,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn major(&self) -> u64 {
        self.major
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<u64> = s
            .split('.')
            .map(str::parse)
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Interval {
    Closed(Version),
    Open(Version),
    Unbounded,
}

impl Interval {
    /// Compares two `Interval`s and returns an ordering. Unfortunately we can't use the Ord trait
    /// beacuse of the extra parameter `lower`.
    pub fn cmp(&self, other: &Interval, lower: bool) -> cmp::Ordering {
        match (self, other) {
            (Interval::Unbounded, Interval::Unbounded) => cmp::Ordering::Equal,
            (Interval::Unbounded, _) => {
                if lower {
                    cmp::Ordering::Less
                } else {
                    cmp::Ordering::Greater
                }
            }
            (_, Interval::Unbounded) => {
                if lower {
                    cmp::Ordering::Greater
                } else {
                    cmp::Ordering::Less
                }
            }
            (Interval::Open(a), Interval::Open(b)) => a.cmp(&b),
            (Interval::Closed(a), Interval::Closed(b)) => a.cmp(&b),
            (Interval::Open(a), Interval::Closed(b)) => {
                if a == b {
                    if lower {
                        cmp::Ordering::Greater
                    } else {
                        cmp::Ordering::Less
                    }
                } else {
                    a.cmp(&b)
                }
            }
            (Interval::Closed(a), Interval::Open(b)) => {
                if a == b {
                    if lower {
                        cmp::Ordering::Less
                    } else {
                        cmp::Ordering::Greater
                    }
                } else {
                    a.cmp(&b)
                }
            }
        }
    }

    pub fn min<'a>(&'a self, other: &'a Interval, lower: bool) -> &'a Interval {
        if self.cmp(other, lower) == cmp::Ordering::Greater {
            other
        } else {
            self
        }
    }

    pub fn max<'a>(&'a self, other: &'a Interval, lower: bool) -> &'a Interval {
        if self.cmp(other, lower) == cmp::Ordering::Less {
            other
        } else {
            self
        }
    }

    pub fn flip(self) -> Interval {
        match self {
            Interval::Closed(v) => Interval::Open(v),
            Interval::Open(v) => Interval::Closed(v),
            Interval::Unbounded => Interval::Unbounded,
        }
    }

    pub fn show(&self, lower: bool) -> String {
        match &self {
            Interval::Unbounded => "".to_string(),
            Interval::Closed(v) => {
                if lower {
                    format!(">={}", v)
                } else {
                    format!("<={}", v)
                }
            }
            Interval::Open(v) => {
                if lower {
                    format!(">{}", v)
                } else {
                    format!("<{}", v)
                }
            }
        }
    }
}

/// A continguous range in which a version can fall into. Syntax for ranges mirrors that of
/// Pub or Cargo. Ranges can accept caret and tilde syntax, as well as less-than/greater-than
/// specifications (just like Cargo). Like Pub, the `any` Range is completely unbounded on
/// both sides. Pre-release `Version`s can satisfy a `Range` iff the `Range`
/// mentions a pre-release `Version` on either bound, or if the `Range` is unbounded on the upper
/// side. Additionally, if a greater-than and/or less-than `Range` also has a `!` after the
/// inequality symbol, the Range will include pre-release versions. `>=! 1.0.0` accepts all
/// pre-releases of 1.0.0, along with the greater versions. `<! 2.0.0` includes pre-releases of
/// 2.0.0. >! and > mean the same thing, as do <=! and <=.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Range {
    lower: Interval,
    upper: Interval,
}

impl Range {
    /// Creates a new `Range`.
    ///
    /// All `Range`s have to be valid, potentially true constraints. If a nonsensical range is
    /// suggested, `None` is returned.
    pub fn new(lower: Interval, upper: Interval) -> Option<Range> {
        match (lower, upper) {
            (Interval::Unbounded, b) => Some(Range {
                lower: Interval::Unbounded,
                upper: b,
            }),
            (a, Interval::Unbounded) => Some(Range {
                lower: a,
                upper: Interval::Unbounded,
            }),
            (Interval::Open(a), Interval::Closed(b)) => {
                if a == b {
                    None
                } else {
                    let (a, b) = (Interval::Open(a), Interval::Closed(b));
                    if a.cmp(&b, true) != cmp::Ordering::Greater {
                        Some(Range { lower: a, upper: b })
                    } else {
                        None
                    }
                }
            }
            (Interval::Closed(a), Interval::Open(b)) => {
                if a == b {
                    None
                } else {
                    let (a, b) = (Interval::Closed(a), Interval::Open(b));
                    if a.cmp(&b, true) != cmp::Ordering::Greater {
                        Some(Range { lower: a, upper: b })
                    } else {
                        None
                    }
                }
            }
            (Interval::Open(a), Interval::Open(b)) => {
                if a == b {
                    None
                } else {
                    let (a, b) = (Interval::Open(a), Interval::Open(b));
                    if a.cmp(&b, true) != cmp::Ordering::Greater {
                        Some(Range { lower: a, upper: b })
                    } else {
                        None
                    }
                }
            }
            (a, b) => {
                if a.cmp(&b, true) != cmp::Ordering::Greater {
                    Some(Range { lower: a, upper: b })
                } else {
                    None
                }
            }
        }
    }

    pub fn any() -> Range {
        let lower = Interval::Unbounded;
        let upper = Interval::Unbounded;

        Range { lower, upper }
    }

    pub fn upper(&self) -> &Interval {
        &self.upper
    }

    pub fn lower(&self) -> &Interval {
        &self.lower
    }

    pub fn take(self) -> (Interval, Interval) {
        (self.lower, self.upper)
    }

    /// Checks if a version is satisfied by this `Range`.
    pub fn satisfies(&self, version: &Version) -> bool {
        // For an upper range, a pre-release will satisfy the upper range if the interval is Open
        // and it either is a prerelease or always accepts prereleases, or if the Interval is
        // Closed or Unbounded. (`<= 2.0.0` includes 2.0.0-alpha, etc. <= is the same as <=!, and
        // as we'll see later, >! is the same as >)

        let satisfies_upper = match &self.upper {
            Open(u) => version < u,
            Closed(u) => version <= u,
            Unbounded => true,
        };
        let satisfies_lower = match &self.lower {
            Open(l) => version > l,
            Closed(l) => version >= l,
            Unbounded => true,
        };

        satisfies_lower && satisfies_upper
    }

    /// Returns the intersection of two `Range`s, or `None` if the two `Range`s are disjoint.
    ///
    /// This function is a method of Range since we will never generate multiple disjoint `Range`s
    /// from an intersection operation.
    fn intersection(&self, other: &Range) -> Option<Range> {
        let lower = self.lower.max(&other.lower, true);
        let upper = self.upper.min(&other.upper, false);

        Range::new(lower.clone(), upper.clone())
    }

    pub fn from(v: &Version, strictness: &Strictness) -> Range {
        let lower = Interval::Closed(*v);
        let upper = match strictness {
            Strictness::Exact => Interval::Closed(*v),
            Strictness::Safe => Interval::Open(Version::new(v.major + 1, v.minor, v.patch)),
            Strictness::Unsafe => Interval::Unbounded,
        };

        Range { lower, upper }
    }
}

impl From<Version> for Range {
    fn from(v: Version) -> Range {
        let lower = Interval::Closed(v);
        let upper = Interval::Closed(v);
        Range { lower, upper }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.lower, &self.upper) {
            (Unbounded, Unbounded) => write!(f, "at any version"),
            (Unbounded, b) => write!(f, "{}", b.show(false)),
            (a, Unbounded) => write!(f, "{}", a.show(true)),
            (Closed(a), Open(b)) => write!(f, "{} <= v < {}", a, b),
            (Closed(a), Closed(b)) if a == b => write!(f, "{}", a),
            (a, b) => write!(f, "{} {}", a.show(true), b.show(false)),
        }
    }
}

/// A set of `Range`s combines to make a `Constraint`. `Constraint`s are the union of multiple
/// `Range`s. Upon manual creation or updating of a `Constraint`, the `Constraint` will unify all
/// of its `Range`s such that all of the `Range`s are disjoint. Unification is eager: it's done
/// whenever the set is modified to keep the internal representation of the set unified at all
/// times (this is useful for converting the `Constraint` to a string, since the `Display` trait
/// doesn't allow mutating self).
///
/// Syntax-wise, a `Constraint` is just a list of comma-separated ranges.
#[derive(Clone, PartialEq, Eq)]
pub struct Constraint {
    set: IndexSet<Range>,
}

impl Constraint {
    /// Creates a new `Constraint` from a set of `Range`s.
    pub fn new(ranges: IndexSet<Range>) -> Constraint {
        let mut c = Constraint { set: ranges };
        c.unify();
        c
    }

    pub fn empty() -> Constraint {
        Constraint { set: indexset!() }
    }

    pub fn any() -> Constraint {
        Range::any().into()
    }

    /// Inserts a `Range` into the set.
    pub fn insert(&mut self, range: Range) {
        self.set.insert(range);
        self.unify();
    }

    /// Borrows the set of `Range`s from this struct, unifying it in the process.
    pub fn retrieve(&self) -> &IndexSet<Range> {
        &self.set
    }

    /// Takes the set of `Range`s from this struct, unifying it in the process.
    pub fn take(self) -> IndexSet<Range> {
        self.set
    }

    /// Unifies all of the ranges in the set such that all of the ranges are disjoint.
    pub fn unify(&mut self) {
        // Note: we take &mut self here because it's more convenient for using with other functions.
        // Turning it back into just self would just be turning sort_by into sorted_by and removing
        // the .cloned() call.
        self.set.sort_by(|a, b| a.lower().cmp(b.lower(), true));

        self.set = self
            .set
            .iter()
            .cloned()
            .coalesce(|a, b| {
                if a.upper().cmp(b.lower(), false) == cmp::Ordering::Greater {
                    let lower = a.take().0;
                    let upper = b.take().1;
                    let r = Range::new(lower, upper).unwrap();
                    Ok(r)
                } else if a.upper().cmp(b.lower(), false) == cmp::Ordering::Equal {
                    if let (Interval::Open(_), Interval::Open(_)) = (a.upper(), b.lower()) {
                        return Err((a, b));
                    }
                    let lower = a.take().0;
                    let upper = b.take().1;
                    let r = Range::new(lower, upper).unwrap();
                    Ok(r)
                } else {
                    let (a2, b2) = (a.clone(), b.clone());
                    let (al, au) = a.take();
                    let (bl, bu) = b.take();
                    if let (Interval::Open(v), Interval::Closed(w)) = (au, bl) {
                        if v == w {
                            let r = Range::new(al, bu).unwrap();
                            Ok(r)
                        } else {
                            Err((a2, b2))
                        }
                    } else {
                        Err((a2, b2))
                    }
                }
            })
            .collect();
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Checks if a `Version` is satisfied by this `Constraint`.
    pub fn satisfies(&self, v: &Version) -> bool {
        self.set.iter().any(|s| s.satisfies(v))
    }

    pub fn intersection(&self, other: &Constraint) -> Constraint {
        let mut set = IndexSet::new();

        for r in &self.set {
            for s in &other.set {
                if let Some(r) = r.intersection(&s) {
                    set.insert(r);
                }
            }
        }

        // We skip unification because we already know that the set will be unified.
        // The only time we might not be unified is during creation or arbitrary insertion.
        Constraint { set }
    }

    pub fn union(&self, other: &Constraint) -> Constraint {
        let mut set = self.set.clone();
        set.extend(other.set.clone());

        Constraint { set }
    }

    pub fn difference(&self, other: &Constraint) -> Constraint {
        let mut set = IndexSet::new();

        for r in &self.set {
            let mut r = r.clone();
            let mut g = true;
            for s in &other.set {
                match r.lower().cmp(s.lower(), true) {
                    cmp::Ordering::Greater => {
                        //------------------//
                        //         [=r=]    //
                        // [==s==]          //
                        //------------------//
                        //        OR        //
                        //------------------//
                        //         [=r=]    //
                        // [===s===]        //
                        //------------------//
                        //        OR        //
                        //------------------//
                        //         [=r=]    //
                        // [====s====]      //
                        //------------------//
                        //        OR        //
                        //------------------//
                        //         [=r=]    //
                        // [======s======]  //
                        //------------------//
                        match r.lower().cmp(s.upper(), false) {
                            cmp::Ordering::Greater => {
                                // Situation 1
                                // Do nothing
                            }
                            cmp::Ordering::Equal => {
                                // Situation 2
                                // If they're the same, the lower bound will always be open no matter
                                // what
                                let lower = if let Interval::Open(_) = s.upper() {
                                    s.upper().clone()
                                } else {
                                    s.upper().clone().flip()
                                };
                                let upper = r.upper().clone();
                                r = Range::new(lower, upper).unwrap();
                            }
                            cmp::Ordering::Less => {
                                // Situation 3 & 4
                                // Special-case for Unbounded because that screws with things
                                if s.upper() == &Interval::Unbounded {
                                    g = false;
                                    break;
                                }
                                let lower = s.upper().clone().flip();
                                let upper = r.upper().clone();
                                // We have to do the if let because in Situation 4 there is no valid
                                // Range
                                if let Some(range) = Range::new(lower, upper) {
                                    r = range;
                                } else {
                                    g = false;
                                    break;
                                }
                            }
                        }
                    }
                    cmp::Ordering::Less => {
                        //------------------//
                        // [=r=]            //
                        //       [==s==]    //
                        //------------------//
                        //        OR        //
                        //------------------//
                        // [==r==]          //
                        //       [==s==]    //
                        //------------------//
                        //        OR        //
                        //------------------//
                        // [====r====]      //
                        //       [==s==]    //
                        //------------------//
                        //        OR        //
                        //------------------//
                        // [======r======]  //
                        //       [==s==]    //
                        //------------------//
                        // Situations 1-3
                        match r.upper().cmp(s.lower(), false) {
                            cmp::Ordering::Less => {
                                // Situation 1
                            }
                            cmp::Ordering::Equal => {
                                // Situation 2
                                let lower = r.lower().clone();
                                let upper = match (r.upper(), s.lower()) {
                                    (Interval::Closed(a), _) => Interval::Open(*a),
                                    (Interval::Open(_), _) => r.upper().clone(),
                                    (_, _) => unreachable!(),
                                };
                                r = Range::new(lower, upper).unwrap();
                            }
                            cmp::Ordering::Greater => {
                                // Situations 3 & 4
                                if r.upper().cmp(s.upper(), false) != cmp::Ordering::Greater {
                                    // Situation 3
                                    let lower = r.lower().clone();
                                    let upper = s.lower().clone().flip();
                                    r = Range::new(lower, upper).unwrap();
                                } else {
                                    // Situation 4
                                    let l1 = r.lower().clone();
                                    let u1 = s.lower().clone().flip();

                                    let l2 = s.upper().clone().flip();
                                    let u2 = r.upper().clone();

                                    // We can do this because we have a guarantee that all ranges
                                    // in a set are disjoint.
                                    set.insert(Range::new(l1, u1).unwrap());
                                    r = Range::new(l2, u2).unwrap();
                                }
                            }
                        }
                    }
                    cmp::Ordering::Equal => {
                        if s.upper() == r.upper() {
                            g = false;
                            break;
                        }

                        // Again, special-casing the Unbounded case.
                        if s.upper() == &Interval::Unbounded {
                            g = false;
                            break;
                        }

                        let lower = s.upper().clone().flip();
                        let upper = r.upper().clone();

                        if let Some(range) = Range::new(lower, upper) {
                            r = range;
                        } else {
                            g = false;
                            break;
                        }
                    }
                }
            }

            if g {
                set.insert(r);
            }
        }

        Constraint { set }
    }

    pub fn complement(&self) -> Constraint {
        Constraint::any().difference(self)
    }

    pub fn relation(&self, other: &Constraint) -> Relation {
        let i = &self.intersection(other);
        if self == other {
            Relation::Equal
        } else if i == other {
            Relation::Superset
        } else if i == self {
            Relation::Subset
        } else if i.set.is_empty() {
            Relation::Disjoint
        } else {
            Relation::Overlapping
        }
    }
}

impl Default for Constraint {
    fn default() -> Self {
        Constraint::any()
    }
}

impl From<Range> for Constraint {
    fn from(r: Range) -> Constraint {
        let mut set = IndexSet::new();
        set.insert(r);

        Constraint { set }
    }
}

impl From<Version> for Constraint {
    fn from(v: Version) -> Constraint {
        let r: Range = v.into();
        r.into()
    }
}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Constraint({})",
            self.set.iter().map(ToString::to_string).join(", ")
        )
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let items: Vec<Range> = self.set.iter().cloned().collect();
        let items: &[Range] = &items;
        match items {
            [Range {
                lower: Interval::Unbounded,
                upper: Interval::Open(l),
            }, Range {
                lower: Interval::Closed(u),
                upper: Interval::Unbounded,
            }] => write!(
                f,
                "at versions other than {}",
                Range::new(Interval::Closed(*l), Interval::Open(*u)).unwrap()
            ),
            _ => write!(f, "{}", items.iter().map(ToString::to_string).join(", ")),
        }
    }
}
