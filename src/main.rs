use ansi_term::{Colour::Red, Style};
use anyhow::Context;
use nixpkgs_check::{checks, Check};
use std::path::PathBuf;
use structopt::StructOpt;

fn error() -> ansi_term::ANSIString<'static> {
    Red.bold().paint("error")
}

fn info() -> ansi_term::ANSIString<'static> {
    Style::new().bold().paint("info")
}

#[derive(StructOpt)]
#[structopt(about = "Run sanity checks on a nixpkgs PR")]
struct Opt {
    /// The revision to check
    #[structopt(default_value = "HEAD")]
    to_check_ref: String,

    /// The base compared to which to check
    #[structopt(default_value = "master")]
    base_ref: String,

    /// The path towards the nixpkgs repository
    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

fn run(opt: Opt) -> anyhow::Result<()> {
    println!(
        "{}: checking changes between merge-base({base}, {checked}) and {checked} in repo {path:?}",
        info(),
        base = opt.base_ref,
        checked = opt.to_check_ref,
        path = opt.repo_path,
    );

    // Open the repo
    let repo = git2::Repository::open(&opt.repo_path)
        .with_context(|| format!("opening the nixpkgs repo {:?}", opt.repo_path))?;

    // Resolve the reference names
    let base_ref = repo
        .resolve_reference_from_short_name(&opt.base_ref)
        .with_context(|| {
            format!(
                "finding reference {:?} in repo {:?}",
                opt.base_ref, opt.repo_path
            )
        })?
        .resolve()
        .context("resolving base reference")?;
    let to_check_ref = repo
        .resolve_reference_from_short_name(&opt.to_check_ref)
        .with_context(|| {
            format!(
                "finding reference {:?} in repo {:?}",
                opt.to_check_ref, opt.repo_path
            )
        })?
        .resolve()
        .context("resolving to-check reference")?;

    // The base reference is actually merge-base(base, to-check)
    let base_oid = repo
        .merge_base(
            base_ref
                .target()
                .context("getting target for base reference")?,
            to_check_ref
                .target()
                .context("getting target for to-check reference")?,
        )
        .context("finding the merge-base of the base branch and the to-check reference")?;
    let base_annotated_commit = repo
        .find_annotated_commit(base_oid)
        .context("creating an annotated commit for the merge-base")?;
    let base_refname = base_annotated_commit
        .refname()
        .context("retrieving refname from annotated commit for the merge-base")?;
    let base_ref = repo
        .resolve_reference_from_short_name(base_refname)
        .context("retrieving reference for the merge-base")?
        .resolve()
        .context("resolving merge-base reference")?;

    // Checkout the worktrees
    let tmpdir = tempfile::tempdir().context("creating temporary directory")?;
    println!("{}: checking out worktrees in {:?}", info(), tmpdir);
    repo.worktree(
        "base",
        tmpdir.path(),
        Some(git2::WorktreeAddOptions::new().reference(Some(&base_ref))),
    )
    .context("creating worktree for the merge-base commit")?;

    std::env::set_current_dir(&opt.repo_path).with_context(|| {
        format!(
            "switching the current directory to nixpkgs root {:?}",
            opt.repo_path
        )
    })?;

    let mut checks = vec![];
    let mut new_checks = vec![Box::new(checks::self_version::Chk::new()) as Box<dyn Check>];

    while !new_checks.is_empty() {
        // TODO: checkout “before”
        for c in new_checks.iter_mut() {
            c.run_before();
        }
        // TODO: checkout “after”
        for c in new_checks.iter_mut() {
            c.run_after();
        }
        let new_new_checks = new_checks
            .iter()
            .flat_map(|c| c.additional_needed_tests().into_iter())
            .collect::<Vec<_>>();
        checks.extend(new_checks.drain(..));
        new_checks = new_new_checks
            .into_iter()
            .filter(|nnc| !checks.iter().any(|c| nnc.uuid() == c.uuid()))
            .collect();
    }

    println!();
    println!();
    println!("Report to be pasted in the PR message");
    println!("-------------------------------------");
    println!();
    for c in checks {
        println!("{}", c.report());
    }

    Ok(())
}

fn main() {
    match run(Opt::from_args()) {
        Ok(()) => (),
        Err(e) => {
            let mut errs = e.chain().rev();
            eprintln!(
                "{}:  {}",
                error(),
                Style::new().bold().paint(format!(
                    "{}",
                    errs.next().expect("got error chain with zero errors")
                ))
            );
            for e in errs {
                eprintln!("  while {}", Style::new().bold().paint(format!("{}", e)));
            }
        }
    }
}
