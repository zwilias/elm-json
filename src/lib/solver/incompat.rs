/* Copyright (c) 2018 David Cao

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
//! Incompatibilities for the dependency resolver.

use super::summary::{self, Summary};
use crate::semver::Constraint;
use colored::Colorize;
use indexmap::{indexmap, IndexMap};
use itertools::Itertools;
use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IncompatibilityCause {
    Dependency,
    Root,
    Unavailable,
    UnknownPackage,
    Derived(usize, usize),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Incompatibility<P>
where
    P: std::hash::Hash + PartialEq + Clone + Eq + fmt::Display,
{
    pub deps: IndexMap<P, Constraint>,
    pub cause: IncompatibilityCause,
}

#[derive(Clone)]
pub enum IncompatMatch<PackageId> {
    Satisfied,
    Almost(PackageId),
    Contradicted,
}

impl<P> Incompatibility<P>
where
    P: summary::PackageId,
{
    pub fn new(deps: IndexMap<P, Constraint>, cause: IncompatibilityCause) -> Self {
        Self { deps, cause }
    }

    pub fn from_dep(a: Summary<P>, b: (P, Constraint)) -> Self {
        let m = indexmap!(
            a.id => a.version.into(),
            b.0 => b.1,
        );

        Self::new(m, IncompatibilityCause::Dependency)
    }

    pub fn deps(&self) -> &IndexMap<P, Constraint> {
        &self.deps
    }

    pub fn derived(&self) -> Option<(usize, usize)> {
        if let IncompatibilityCause::Derived(l, r) = self.cause {
            Some((l, r))
        } else {
            None
        }
    }

    pub fn is_derived(&self) -> bool {
        self.derived().is_some()
    }

    pub fn cause(&self) -> IncompatibilityCause {
        self.cause
    }

    pub fn show(&self) -> String {
        match self.cause {
            IncompatibilityCause::Dependency => {
                assert!(self.deps.len() == 2);
                let depender = self.deps.get_index(0).unwrap();
                let dependee = self.deps.get_index(1).unwrap();
                format!(
                    "{} depends on {}",
                    Self::show_pkg(depender.0, depender.1),
                    Self::show_pkg(dependee.0, &dependee.1.complement())
                )
            }
            IncompatibilityCause::Unavailable => {
                assert!(self.deps.len() == 1);
                let package = self.deps.get_index(0).unwrap();
                format!("{} is unavailable", Self::show_pkg(package.0, package.1))
            }
            IncompatibilityCause::UnknownPackage => {
                assert!(self.deps.len() == 1);
                let package = self.deps.get_index(0).unwrap();
                format!("{} does not appear to exist", package.0.to_string().bold())
            }
            IncompatibilityCause::Root => "the root package was chosen".to_string(),
            IncompatibilityCause::Derived(_, _) => {
                if self.deps.len() == 1 {
                    "no valid set of package versions could be found".to_string()
                } else if self.deps.len() == 2 {
                    let p1 = self.deps.get_index(0).unwrap();
                    let p2 = self.deps.get_index(1).unwrap();
                    format!(
                        "{} is incompatible with {}",
                        Self::show_pkg(p1.0, p1.1),
                        Self::show_pkg(p2.0, p2.1)
                    )
                } else {
                    format!(
                        "one of {} must be false",
                        self.deps
                            .iter()
                            .map(|(k, v)| Self::show_pkg(k, v))
                            .join("; ")
                    )
                }
            }
        }
    }

    fn show_pkg(pkg: &P, constraint: &Constraint) -> String {
        if pkg.is_root() {
            "this project".to_string()
        } else {
            format!("{} {}", pkg.to_string().bold(), constraint)
        }
    }

    // TODO: Actually special-case stuff to look nicer.
    pub fn show_combine(
        &self,
        other: &Self,
        self_linum: Option<u16>,
        other_linum: Option<u16>,
    ) -> String {
        if let Some(b) = self.show_combine_same(other, self_linum) {
            return b;
        }

        let mut buf = self.show();
        if let Some(l) = self_linum {
            buf.push_str(" (");
            buf.push_str(&l.to_string());
            buf.push(')');
        }
        buf.push_str(" and ");
        buf.push_str(&other.show());
        if let Some(l) = other_linum {
            buf.push_str(" (");
            buf.push_str(&l.to_string());
            buf.push(')');
        }

        buf
    }

    fn show_combine_same(&self, other: &Self, self_linum: Option<u16>) -> Option<String> {
        if self == other {
            let mut buf = self.show();
            if let Some(l) = self_linum {
                buf.push_str(" (");
                buf.push_str(&l.to_string());
                buf.push(')');
            }
            Some(buf)
        } else {
            None
        }
    }
}

impl<P> fmt::Debug for Incompatibility<P>
where
    P: summary::PackageId,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Incompatibility::{:?}({})",
            self.cause,
            self.deps
                .iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .join("; "),
        )
    }
}
