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
pub mod assignment;
pub mod incompat;
pub mod retriever;
pub mod summary;

pub use self::{
    assignment::{Assignment, AssignmentType},
    incompat::{IncompatMatch, Incompatibility, IncompatibilityCause},
    retriever::Retriever,
    summary::Summary,
};
use crate::semver::{Constraint, Relation, Version};
use anyhow::{bail, Error};
use indexmap::{indexmap, indexset, IndexMap};
use petgraph::{
    self,
    graphmap::{DiGraphMap, NodeTrait},
    Direction,
};
use std::{cmp, collections::VecDeque};
use tracing::{info, trace};

pub type Graph<T> = petgraph::Graph<T, ()>;

#[derive(Debug)]
pub struct Resolver<'ret, R: Retriever> {
    step: u16,
    level: u16,
    assignments: Vec<Assignment<R::PackageId>>,
    decisions: IndexMap<R::PackageId, Version>,
    derivations: IndexMap<R::PackageId, (bool, Constraint)>,
    incompats: Vec<Incompatibility<R::PackageId>>,
    incompat_ixs: IndexMap<R::PackageId, Vec<usize>>,
    retriever: &'ret mut R,
}

impl<'ret, R> Resolver<'ret, R>
where
    R: Retriever,
{
    pub fn new(retriever: &'ret mut R) -> Self {
        let step = 1;
        let level = 0;
        let assignments = vec![];
        let incompats = vec![];
        let incompat_ixs = indexmap!();
        let decisions = indexmap!();
        let derivations = indexmap!();
        Resolver {
            step,
            level,
            assignments,
            incompats,
            incompat_ixs,
            decisions,
            derivations,
            retriever,
        }
    }

    pub fn solve(self) -> Result<Graph<Summary<R::PackageId>>, Error> {
        let mut s = self;

        info!("beginning dependency resolution");
        let r = s.solve_loop();

        if r.is_err() {
            info!("solve failed");
            bail!("{}", &s.pp_error(s.incompats.len() - 1))
        } else {
            info!("solve successful");
            r
        }
    }

    fn solve_loop(&mut self) -> Result<Graph<Summary<R::PackageId>>, Error> {
        let c: Constraint = self.retriever.root().version().into();
        let pkgs = indexmap!(self.retriever.root().id() => c.complement());
        self.incompatibility(pkgs, IncompatibilityCause::Root);

        let mut next = Some(self.retriever.root().id());
        while let Some(n) = next {
            self.propagate(n)?;
            next = self.choose_pkg_version();
        }

        // To build the tree, we're gonna go through all our dependencies and get their deps,
        // and build our tree with a BFS. It's one last inefficient process before we have our
        // nice resolution... oh well.
        let mut tree = petgraph::Graph::new();
        let mut set = indexmap!();
        let mut q = VecDeque::new();
        let root = self.retriever.root();
        let root_node = tree.add_node(root.clone());
        set.insert(root, root_node);
        q.push_back(root_node);

        while let Some(pid) = q.pop_front() {
            // At this point, we know there has to be dependencies for these packages.
            let deps = self.retriever.incompats(&tree[pid]).unwrap();
            for inc in deps {
                let pkg = inc.deps.get_index(1).unwrap().0;
                let ver = &self.decisions[pkg];
                let sum = Summary::new(pkg.clone(), *ver);

                let nix = if set.contains_key(&sum) {
                    set[&sum]
                // We don't push to q here because if it's already in the set, the else must
                // have run before, meaning it's already been in the q.
                } else {
                    let nix = tree.add_node(sum.clone());
                    set.insert(sum, nix);
                    q.push_back(nix);
                    nix
                };

                tree.add_edge(pid, nix, ());
            }
        }

        Ok(tree)
    }

    // 1: Unit propagation
    fn propagate(&mut self, pkg: R::PackageId) -> Result<(), Error> {
        let mut changed = indexset!(pkg);

        while let Some(package) = changed.pop() {
            // Yeah, I hate cloning too, but unfortunately it's necessary here
            if let Some(icixs) = self.incompat_ixs.clone().get(&package) {
                'f: for icix in icixs.iter().rev() {
                    let res = self.propagate_incompat(*icix);
                    match res {
                        IncompatMatch::Almost(name) => {
                            changed.insert(name);
                        }
                        IncompatMatch::Satisfied => {
                            let root = self.resolve_conflict(*icix)?;
                            changed.clear();
                            if let IncompatMatch::Almost(name) = self.propagate_incompat(root) {
                                changed.insert(name);
                            } else {
                                unreachable!();
                            }
                            break 'f;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn propagate_incompat(&mut self, icix: usize) -> IncompatMatch<R::PackageId> {
        // Yes, we're cloning again. I'm sorry.
        let inc = &self.incompats[icix].clone();
        let mut unsatis = None;
        let cause = inc.cause();

        for (ix, (pkg, con)) in inc.deps().iter().enumerate() {
            let relation = self.relation(pkg, con);
            let positive = (ix == 1 && cause == IncompatibilityCause::Dependency)
                || cause == IncompatibilityCause::Root;
            // We have to special-case the "any" dependency because the any derivation is a superset of the null set, which would
            // result in continuous "Almost"s if a package only depends on any version of one other package.
            if relation == Relation::Disjoint
                || (con.is_empty() && self.derivations.get(pkg).is_some())
            {
                return IncompatMatch::Contradicted;
            } else if relation != Relation::Subset && relation != Relation::Equal {
                if unsatis.is_none() {
                    // Any derivation other than one we got from a Dependency incompatibility is a
                    // negative incompatibility; it doesn't necessarily require that a package
                    // exists, only that certain versions of it don't exist.
                    // Once a package has a positive derivation, it stays positive *forever*
                    unsatis = Some((pkg, con, positive));
                } else {
                    // We can't deduce anything. This should prolly be "None" instead of
                    // `Contradicted`, but oh well.
                    return IncompatMatch::Contradicted;
                }
            }
        }

        if let Some((pkg, con, positive)) = unsatis {
            self.derivation(pkg.clone(), con.complement(), icix, positive);
            IncompatMatch::Almost(pkg.clone())
        } else {
            IncompatMatch::Satisfied
        }
    }

    fn relation(&self, pkg: &R::PackageId, con: &Constraint) -> Relation {
        if let Some(c) = self.derivations.get(pkg) {
            c.1.relation(con)
        } else {
            // If we can't find anything, that means it allows all versions!
            // This is different from Constraints, in which not having anything means no solution
            Relation::Superset
        }
    }

    // 2: Conflict resolution
    // This function is basically the only reason why we need NLL; we're doing immutable borrows
    // with satisfier, but mutable ones with backtrack & incompatibility.
    fn resolve_conflict(&mut self, inc: usize) -> Result<usize, Error> {
        let mut inc = inc;
        let mut new_incompatibility = false;
        trace!("entering conflict resolution");
        while !self.is_failure(&self.incompats[inc]) {
            let i = self.incompats[inc].clone();
            let mut most_recent_term: Option<(&R::PackageId, &Constraint)> = None;
            let mut most_recent_satisfier: Option<&Assignment<R::PackageId>> = None;
            let mut difference: Option<(&R::PackageId, Constraint)> = None;

            let mut previous_satisfier_level = 1;
            for (pkg, c) in i.deps() {
                // We unwrap here because if this incompatibility is satisfied, it must have
                // been satisfied at some point before...
                let satisfier = self.satisfier(pkg, c).unwrap();

                match most_recent_satisfier {
                    Some(a) => {
                        if a.step() < satisfier.step() {
                            previous_satisfier_level =
                                cmp::max(previous_satisfier_level, a.level());
                            most_recent_term = Some((pkg, c));
                            most_recent_satisfier = Some(satisfier);
                            difference = None;
                        } else {
                            previous_satisfier_level =
                                cmp::max(previous_satisfier_level, satisfier.level());
                        }
                    }
                    None => {
                        most_recent_term = Some((pkg, c));
                        most_recent_satisfier = Some(satisfier);
                    }
                }

                // By this point, most_recent_satisfier and _term will definitely be assigned to.
                let most_recent_satisfier = most_recent_satisfier.unwrap();
                let most_recent_term = most_recent_term.unwrap();
                if most_recent_term == (pkg, c) {
                    difference = {
                        let diff = most_recent_satisfier
                            .constraint()
                            .difference(most_recent_term.1);

                        if diff == Constraint::empty() {
                            None
                        } else {
                            Some((pkg, diff))
                        }
                    };

                    if let Some((pkg, diff)) = &difference {
                        previous_satisfier_level = cmp::max(
                            previous_satisfier_level,
                            self.satisfier(pkg, &diff.complement()).unwrap().level(),
                        );
                    }
                }
            }

            let most_recent_satisfier = most_recent_satisfier.unwrap();
            let most_recent_term = most_recent_term.unwrap();
            if previous_satisfier_level < most_recent_satisfier.level()
                || most_recent_satisfier.cause() == None
            {
                self.backtrack(previous_satisfier_level);
                if new_incompatibility {
                    self.incompat_ixs(inc);
                }
                return Ok(inc);
            }

            // newterms etc
            let cause = self.incompats[most_recent_satisfier.cause().unwrap()].clone();
            let mut new_terms: IndexMap<R::PackageId, Constraint> = IndexMap::new()
                .into_iter()
                .chain(
                    i.deps()
                        .clone()
                        .into_iter()
                        .filter(|t| (&t.0, &t.1) != most_recent_term),
                )
                .chain(
                    cause
                        .deps()
                        .clone()
                        .into_iter()
                        .filter(|t| &t.0 != most_recent_satisfier.pkg()),
                )
                .collect();

            if let Some((pkg, diff)) = difference {
                new_terms.insert(pkg.clone(), diff.complement());
            }

            let new_i = Incompatibility::new(
                new_terms,
                IncompatibilityCause::Derived(inc, most_recent_satisfier.cause().unwrap()),
            );
            // What Pub does is just add the current incompatibility directly as
            // a cause of the new incompatibility. Unfortunately, we don't want
            // to be copying *that* much, so instead we just add the
            // incompatibility to the global cache. I'm not entirely sure if
            // this is totally correct, but oh well.
            inc = self.incompats.len();
            self.incompats.push(new_i);
            new_incompatibility = true;
        }

        bail!("No conflict resultion found")
    }

    fn backtrack(&mut self, previous_satisfier_level: u16) {
        let mut packages = indexset!();
        trace!(
            from_level = self.level,
            to_level = previous_satisfier_level,
            "backtracking"
        );
        self.level = previous_satisfier_level;

        loop {
            let last = self.assignments.pop().unwrap();
            if last.level() > previous_satisfier_level {
                self.step -= 1;
                packages.insert(last.pkg().clone());
            } else {
                self.assignments.push(last);
                break;
            }
        }

        // Re-compute the constraint for these packages.
        for package in &packages {
            self.decisions.remove(package);
            self.derivations.remove(package);
        }

        let assignments = self.assignments.clone();
        for assignment in assignments {
            if packages.contains(assignment.pkg()) {
                self.register(&assignment);
            }
        }
    }

    fn is_failure(&self, inc: &Incompatibility<R::PackageId>) -> bool {
        inc.deps().is_empty()
            || (inc.deps().len() == 1 && {
                let left: &R::PackageId = inc.deps().get_index(0).unwrap().0;
                let right: &R::PackageId = &self.retriever.root().id();
                left == right
            })
    }

    // 3: Decision making
    fn choose_pkg_version(&mut self) -> Option<R::PackageId> {
        let mut unsatisfied = self
            .derivations
            .iter()
            .filter_map(|(k, v)| {
                if v.0 && !self.decisions.contains_key(k) {
                    Some((k, &v.1))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if unsatisfied.is_empty() {
            None
        } else {
            // We want to find the unsatisfied package with the fewest available versions.
            unsatisfied.sort_by(|a, b| {
                // Reversing the comparison will put the items with the least
                // versions at the end, which is more efficient for popping
                self.retriever
                    .count_versions(a.0)
                    .cmp(&self.retriever.count_versions(b.0))
                    .reverse()
            });
            let package = unsatisfied.pop().unwrap();
            let best = self.retriever.best(package.0, package.1);
            let res = Some(package.0.clone());
            match best {
                Ok(best) => {
                    let sum = Summary::new(package.0.clone(), best);
                    // We know the package exists, so unwrapping here is fine
                    let incompats = self.retriever.incompats(&sum).unwrap();
                    let mut conflict = false;
                    for ic in incompats {
                        conflict = conflict
                            || ic
                                .deps
                                .iter()
                                .map(|(k, v)| {
                                    *k == sum.id()
                                        || self.relation(k, v) == Relation::Subset
                                        || self.relation(k, v) == Relation::Equal
                                })
                                .all(|b| b);
                        self.incompatibility(ic.deps, ic.cause);
                    }
                    if !conflict {
                        self.decision(sum.id, best);
                    }
                }
                Err(e) => {
                    info!("Failed to add package {} {}: {}", package.0, package.1, e);
                    let pkgs = indexmap!(
                        package.0.clone() => package.1.clone()
                    );
                    if self.retriever.count_versions(package.0) == 0 {
                        self.incompatibility(pkgs, IncompatibilityCause::UnknownPackage);
                    } else {
                        self.incompatibility(pkgs, IncompatibilityCause::Unavailable);
                    }
                }
            }
            res
        }
    }

    fn satisfier(&self, pkg: &R::PackageId, con: &Constraint) -> Option<&Assignment<R::PackageId>> {
        let mut assigned_term = Constraint::any();

        for assignment in &self.assignments {
            if assignment.pkg() != pkg {
                continue;
            }

            assigned_term = assigned_term.intersection(&assignment.constraint());

            if assigned_term.relation(con) == Relation::Subset
                || assigned_term.relation(con) == Relation::Equal
            {
                return Some(assignment);
            }
        }

        None
    }

    // 4: Error reporting
    // cause things go bad
    fn pp_error(&self, root_icix: usize) -> String {
        let mut linum: IndexMap<usize, u16> = indexmap!();
        let mut cur_linum = 1;
        let mut ics = DiGraphMap::<usize, ()>::new();
        for (ix, i) in self.incompats.iter().enumerate() {
            ics.add_node(ix);
            if let Some((l, r)) = i.derived() {
                ics.add_edge(ix, l, ());
                ics.add_edge(ix, r, ());
            }
        }

        let mut s = String::new();
        self.pp_err_recur(root_icix, &ics, &mut linum, &mut cur_linum, &mut s);

        s
    }

    #[allow(clippy::unnecessary_unwrap)]
    fn pp_err_recur(
        &self,
        icix: usize,
        ics: &DiGraphMap<usize, ()>,
        linum: &mut IndexMap<usize, u16>,
        cur_linum: &mut u16,
        out: &mut String,
    ) {
        let root = &self.incompats[icix];
        let (left_ix, right_ix) = if let Some(r) = get_two(ics, icix) {
            r
        } else {
            // This case only happens if the root package is inaccessible
            out.push_str("An error occurred with the root package");
            return;
        };
        let (left, right) = (&self.incompats[left_ix], &self.incompats[right_ix]);

        match (get_two(ics, left_ix), get_two(ics, right_ix)) {
            (Some((l1, l2)), Some((r1, r2))) => {
                // Case 1 in the Pubgrub doc
                let left_line = linum.get(&left_ix).cloned();
                let right_line = linum.get(&right_ix).cloned();

                match (left_line, right_line) {
                    (Some(l), Some(r)) => {
                        out.push_str("Because ");
                        out.push_str(&left.show_combine(right, Some(l), Some(r)));
                    }
                    (Some(l), None) => {
                        self.pp_err_recur(right_ix, ics, linum, cur_linum, out);
                        out.push_str("\nAnd because ");
                        out.push_str(&left.show());
                        out.push_str(" (");
                        out.push_str(&l.to_string());
                        out.push(')');
                    }
                    (None, Some(r)) => {
                        self.pp_err_recur(right_ix, ics, linum, cur_linum, out);
                        out.push_str("\nAnd because ");
                        out.push_str(&right.show());
                        out.push_str(" (");
                        out.push_str(&r.to_string());
                        out.push(')');
                    }
                    (None, None) => {
                        match (
                            get_two(ics, l1),
                            get_two(ics, l2),
                            get_two(ics, r1),
                            get_two(ics, r2),
                        ) {
                            (Some(_), Some(_), Some(_), Some(_))
                            | (Some(_), Some(_), None, None) => {
                                self.pp_err_recur(right_ix, ics, linum, cur_linum, out);
                                self.pp_err_recur(left_ix, ics, linum, cur_linum, out);
                                out.push_str("Thus");
                            }
                            (None, None, Some(_), Some(_)) => {
                                self.pp_err_recur(left_ix, ics, linum, cur_linum, out);
                                self.pp_err_recur(right_ix, ics, linum, cur_linum, out);
                                out.push_str("Thus");
                            }
                            _ => {
                                self.pp_err_recur(left_ix, ics, linum, cur_linum, out);
                                if !linum.contains_key(&left_ix) {
                                    // Remove the \n from before
                                    out.pop();
                                    out.push_str(" (");
                                    out.push_str(&cur_linum.to_string());
                                    out.push(')');
                                    linum.insert(icix, *cur_linum);
                                    *cur_linum += 1;
                                    out.push('\n');
                                }
                                out.push('\n');
                                self.pp_err_recur(right_ix, ics, linum, cur_linum, out);

                                // TODO: This just feels wrong
                                // "Associate this line number with the first cause"
                                // Remove the \n from before
                                out.pop();
                                out.push_str(" (");
                                out.push_str(&cur_linum.to_string());
                                out.push(')');
                                linum.insert(icix, *cur_linum);
                                *cur_linum += 1;
                                out.push('\n');

                                out.push_str("\nAnd because ");
                                out.push_str(&left.show());
                            }
                        }
                    }
                }
            }
            (None, None) => {
                // Case 3 in the Pubgrub doc: both are external.
                out.push_str("Because ");
                out.push_str(&left.show_combine(right, None, None));
            }
            (ld, rd) => {
                let derived_ix = match (ld, rd) {
                    (Some(_), None) => left_ix,
                    (None, Some(_)) => right_ix,
                    _ => unreachable!(),
                };

                let (derived, external) = match (ld, rd) {
                    (Some(_), None) => (left, right),
                    (None, Some(_)) => (right, left),
                    _ => unreachable!(),
                };

                if linum.contains_key(&derived_ix) {
                    let l = linum[&derived_ix];
                    out.push_str("Because ");
                    out.push_str(&external.show_combine(derived, None, Some(l)));
                } else {
                    let d2 = get_two(ics, derived_ix);
                    if d2.is_some()
                        && ((get_two(ics, d2.unwrap().0).is_some()
                            && !linum.contains_key(&d2.unwrap().0))
                            ^ (get_two(ics, d2.unwrap().1).is_some()
                                && !linum.contains_key(&d2.unwrap().1)))
                    {
                        let a = &self.incompats[d2.unwrap().0];
                        let b = &self.incompats[d2.unwrap().1];
                        let prior_derived_ix = match (a.derived(), b.derived()) {
                            (Some(_), None) => d2.unwrap().0,
                            (None, Some(_)) => d2.unwrap().1,
                            _ => unreachable!(),
                        };
                        let prior_external = match (a.derived(), b.derived()) {
                            (Some(_), None) => a,
                            (None, Some(_)) => b,
                            _ => unreachable!(),
                        };

                        self.pp_err_recur(prior_derived_ix, ics, linum, cur_linum, out);
                        out.push_str("\nAnd because ");
                        out.push_str(&prior_external.show_combine(external, None, None));
                    } else {
                        self.pp_err_recur(derived_ix, ics, linum, cur_linum, out);
                        out.push_str("\nAnd because ");
                        out.push_str(&external.show());
                    }
                }
            }
        }

        out.push_str(", ");
        out.push_str(&root.show());
        out.push('.');
        if ics.neighbors_directed(icix, Direction::Incoming).count() >= 2 {
            out.push_str(" (");
            out.push_str(&cur_linum.to_string());
            out.push(')');
            linum.insert(icix, *cur_linum);
            *cur_linum += 1;
        }
        out.push('\n');
    }

    fn register(&mut self, a: &Assignment<R::PackageId>) {
        match a.ty() {
            AssignmentType::Decision { version } => {
                self.decisions.insert(a.pkg().clone(), *version);
                self.derivations
                    .insert(a.pkg().clone(), (true, (*version).into()));
            }
            AssignmentType::Derivation {
                cause: _cause,
                constraint,
                positive,
            } => {
                if !self.derivations.contains_key(a.pkg()) {
                    self.derivations
                        .insert(a.pkg().clone(), (*positive, constraint.clone()));
                } else {
                    let old = self.derivations.get_mut(a.pkg()).unwrap();
                    *old = (old.0 || *positive, old.1.intersection(constraint));
                }
            }
        }
    }

    fn decision(&mut self, pkg: R::PackageId, version: Version) {
        self.level += 1;
        trace!(
            step = self.step,
            level = self.level,
            package = %pkg,
            version = %version,
            "Decision"
        );
        let a = Assignment::new(
            self.step,
            self.level,
            pkg,
            AssignmentType::Decision { version },
        );
        self.register(&a);
        self.assignments.push(a);
        self.step += 1;
    }

    fn derivation(&mut self, pkg: R::PackageId, c: Constraint, cause: usize, positive: bool) {
        trace!(
            step = self.step,
            level = self.level,
            package = %pkg,
            constraint = %c,
            "Derivation",
        );
        let a = Assignment::new(
            self.step,
            self.level,
            pkg,
            AssignmentType::Derivation {
                constraint: c,
                cause,
                positive,
            },
        );
        self.register(&a);
        self.assignments.push(a);
        self.step += 1;
    }

    fn incompatibility(
        &mut self,
        pkgs: IndexMap<R::PackageId, Constraint>,
        cause: IncompatibilityCause,
    ) -> usize {
        let new_ix = self.incompats.len();
        let ic = Incompatibility::new(pkgs, cause);
        trace!(incompat = ?ic, "New incompat");
        self.incompats.push(ic);
        self.incompat_ixs(new_ix);

        new_ix
    }

    fn incompat_ixs(&mut self, icix: usize) {
        let ic = &self.incompats[icix];
        for (n, _) in ic.deps() {
            self.incompat_ixs
                .entry(n.clone())
                .or_insert_with(Vec::new)
                .push(icix);
        }
    }
}

fn get_two<T: NodeTrait, E>(graph: &DiGraphMap<T, E>, root: T) -> Option<(T, T)> {
    let xs = graph
        .neighbors_directed(root, Direction::Outgoing)
        .collect::<Vec<_>>();
    if xs.len() == 2 {
        Some((xs[0], xs[1]))
    } else if xs.len() == 1 {
        Some((xs[0], xs[0]))
    } else {
        None
    }
}
