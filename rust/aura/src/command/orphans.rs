//! All functionality involving the `-O` command.

use crate::{a, green, yellow};
use alpm::{Alpm, PackageReason, TransFlag};
use aura_arch as arch;
use colored::*;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;
use std::collections::HashSet;
use ubyte::ToByteUnit;

pub enum Error {
    Alpm(alpm::Error),
    Readline(rustyline::error::ReadlineError),
    Sudo(crate::utils::SudoError),
    Cancelled,
    NoneExist,
}

impl From<crate::utils::SudoError> for Error {
    fn from(v: crate::utils::SudoError) -> Self {
        Self::Sudo(v)
    }
}

impl From<rustyline::error::ReadlineError> for Error {
    fn from(v: rustyline::error::ReadlineError) -> Self {
        Self::Readline(v)
    }
}

impl From<alpm::Error> for Error {
    fn from(v: alpm::Error) -> Self {
        Self::Alpm(v)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Alpm(e) => write!(f, "{}", e),
            Error::Readline(e) => write!(f, "{}", e),
            Error::Sudo(e) => write!(f, "{}", e),
            Error::NoneExist => write!(f, "No such packages exist."),
            Error::Cancelled => write!(f, "Action cancelled."),
        }
    }
}

/// Print the name of each orphaned package.
pub(crate) fn list(alpm: &Alpm) {
    arch::orphans(&alpm).for_each(|o| println!("{}", o.name()))
}

/// Sets a package's install reason to "as explicit". An alias for `-D --asexplicit`.
pub(crate) fn adopt(
    alpm: &Alpm,
    fll: FluentLanguageLoader,
    packages: Vec<String>,
) -> Result<(), Error> {
    crate::utils::sudo()?;

    let db = alpm.localdb();
    let reals: Vec<_> = packages
        .into_iter()
        .filter_map(|p| db.pkg(p).ok())
        .collect();

    // Exit early if no real packages were given.
    if reals.is_empty() {
        return Err(Error::NoneExist);
    }

    for mut p in reals {
        p.set_reason(PackageReason::Explicit)?;
        green!(fll, "O-adopt", package = p.name());
    }

    Ok(())
}

/// Uninstall all orphan packages.
///
/// Will fail if the process does not have permission to create the lockfile,
/// which usually lives in a root-owned directory.
pub(crate) fn remove(alpm: &mut Alpm, fll: FluentLanguageLoader) -> Result<(), Error> {
    crate::utils::sudo()?;

    // Check for orphans.
    let orphans: Vec<_> = arch::orphans(alpm).collect();
    if !orphans.is_empty() {
        // Copy the name of each original orphan.
        let names: HashSet<_> = orphans.iter().map(|p| p.name().to_string()).collect();

        // Initialize the transaction.
        let mut flag = TransFlag::RECURSE;
        flag.insert(TransFlag::UNNEEDED);
        alpm.trans_init(flag)?;

        for p in orphans {
            alpm.trans_remove_pkg(p)?;
        }

        // Advance the transaction, calculating the effects of the TransFlags.
        alpm.trans_prepare().map_err(|(_, e)| Error::Alpm(e))?;

        // Notify the user of the results.
        let removal = alpm.trans_remove();
        let longest = removal.iter().map(|p| p.name().len()).max().unwrap_or(0);
        yellow!(fll, "O-abandon");
        println!();
        for p in removal {
            let size = format!("{}", p.isize().bytes());
            if names.contains(p.name()) {
                print!("  {:w$} ", p.name().cyan(), w = longest);
                println!("{:>9}", size);
            } else {
                println!("  {:w$} {:>9}", p.name(), size, w = longest);
            }
        }
        println!("  {:-<w$}", "-".magenta(), w = longest + 10);
        let total: i64 = removal.iter().map(|p| p.isize()).sum();
        let size = format!("{}", total.bytes());
        println!("  {:w$} {:>9}\n", "Total", size, w = longest);

        // Proceed with the removal if the user accepts.
        let msg = format!("{} {} ", fl!(fll, "proceed"), fl!(fll, "proceed-yes"));
        crate::utils::prompt(&a!(msg)).ok_or(Error::Cancelled)?;
        alpm.trans_commit().map_err(|(_, e)| Error::Alpm(e))?;
        alpm.trans_release()?;
        green!(fll, "common-done");
    }

    Ok(())
}
