use colored::Colorize;
use std::{cmp::Ordering, fmt};

pub enum Kind {
    Regular,
    Test,
    Direct,
    DirectTest,
    Indirect,
    IndirectTest,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Kind::Regular => Ok(()),
            Kind::Test => write!(f, "{} ", "test".bold()),
            Kind::Direct => write!(f, "{} ", "direct".bold()),
            Kind::DirectTest => write!(f, "{} ", "direct test".bold()),
            Kind::Indirect => write!(f, "{} ", "indirect".bold()),
            Kind::IndirectTest => write!(f, "{} ", "indirect test".bold()),
        }
    }
}

pub fn show<'a, L, R, K: 'a, T: 'a>(kind: Kind, left: L, right: R)
where
    L: IntoIterator<Item = (&'a K, &'a T)>,
    R: IntoIterator<Item = (&'a K, &'a T)>,
    T: Eq + std::fmt::Display + Sized + Copy,
    K: std::fmt::Display + Ord + Clone,
{
    let it = Diff::new(left, right);
    if !it.is_empty() {
        println!("I want to make some changes to your {}dependencies\n", kind);
        it.print();
        println!();
    }
}

impl<'a, K, T> Diff<'a, K, T>
where
    T: Sized + Eq + Copy + std::fmt::Display,
    K: std::fmt::Display + Ord + Clone,
{
    pub fn new<L, R>(left: L, right: R) -> Self
    where
        L: IntoIterator<Item = (&'a K, &'a T)>,
        R: IntoIterator<Item = (&'a K, &'a T)>,
    {
        let mut only_left = Vec::new();
        let mut only_right = Vec::new();
        let mut changed = Vec::new();

        let mut iter_left = left.into_iter();
        let mut iter_right = right.into_iter();

        let mut left = iter_left.next();
        let mut right = iter_right.next();

        while let (Some((left_name, left_version)), Some((right_name, right_version))) =
            (left, right)
        {
            match left_name.cmp(right_name) {
                Ordering::Equal => {
                    if left_version != right_version {
                        changed.push((left_name, left_version, right_version))
                    }

                    left = iter_left.next();
                    right = iter_right.next();
                }
                Ordering::Less => {
                    only_left.push((left_name, left_version));
                    left = iter_left.next();
                }
                Ordering::Greater => {
                    only_right.push((right_name, right_version));
                    right = iter_right.next();
                }
            }
        }

        while let Some((name, version)) = left {
            only_left.push((name, version));
            left = iter_left.next();
        }

        while let Some((name, version)) = right {
            only_right.push((name, version));
            right = iter_right.next();
        }

        Self {
            only_left,
            only_right,
            changed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.only_left.is_empty() && self.only_right.is_empty() && self.changed.is_empty()
    }

    pub fn print(&self) {
        for (k, v) in &self.only_left {
            println!("- {} {} {}", "[DEL]".yellow(), k, v);
        }

        for (k, o, n) in &self.changed {
            println!("- {} {} {} -> {}", "[CHG]".blue(), k, o, n);
        }

        for (k, v) in &self.only_right {
            println!("- {} {} {}", "[ADD]".green(), k, v);
        }
    }
}

struct Diff<'a, K, T>
where
    K: Ord + std::fmt::Display + Clone,
    T: Eq + Sized + Copy + std::fmt::Display,
{
    only_left: Vec<(&'a K, &'a T)>,
    only_right: Vec<(&'a K, &'a T)>,
    changed: Vec<(&'a K, &'a T, &'a T)>,
}
