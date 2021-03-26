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
    let base_obj = repo.revparse_single(&opt.base_ref).with_context(|| {
        format!(
            "finding reference {:?} in repo {:?}",
            opt.base_ref, opt.repo_path
        )
    })?;
    let to_check_obj = repo.revparse_single(&opt.to_check_ref).with_context(|| {
        format!(
            "finding reference {:?} in repo {:?}",
            opt.to_check_ref, opt.repo_path
        )
    })?;

    // The base reference is actually merge-base(base, to-check)
    let base_oid = repo
        .merge_base(base_obj.id(), to_check_obj.id())
        .context("finding the merge-base of the base reference and the to-check reference")?;
    let base_obj = repo
        .find_object(base_oid, None)
        .context("recovering an object from the merge-base oid")?;
    println!(
        "{}: merge-base is {}",
        info(),
        base_obj
            .short_id()
            .context("retrieving short id for merge-base")?
            .as_str()
            .expect("short id is not utf-8"),
    );

    // Checkout the commits in worktrees
    let tempdir = tempfile::tempdir().context("creating temporary directory")?;
    let base_path = tempdir.path().join("base");
    let to_check_path = tempdir.path().join("to-check");
    let initial_cwd = std::env::current_dir().context("recovering current working directory")?;

    println!("{}: checking out base worktree at {:?}", info(), base_path);
    let base_wt = repo
        .worktree(
            uuid::Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer()),
            &base_path,
            None,
        )
        .context("creating worktree for base commit")?;
    let base_repo = git2::Repository::open_from_worktree(&base_wt)
        .context("opening base worktree as a repository")?;
    base_repo
        .checkout_tree(
            &base_repo
                .find_object(base_obj.id(), None)
                .context("converting repo object to worktree object")?,
            None,
        )
        .context("checking out base commit in worktree")?;
    println!(
        "{}: checking out to-check worktree at {:?}",
        info(),
        to_check_path,
    );

    let to_check_wt = repo
        .worktree(
            uuid::Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer()),
            &to_check_path,
            None,
        )
        .context("creating worktree for to-check commit")?;
    let to_check_repo = git2::Repository::open_from_worktree(&to_check_wt)
        .context("opening base worktree as a repository")?;
    to_check_repo
        .checkout_tree(
            &to_check_repo
                .find_object(to_check_obj.id(), None)
                .context("converting repo object to worktree object")?,
            None,
        )
        .context("checking out to-check commit in worktree")?;

    let mut checks = vec![];
    let mut new_checks = vec![Box::new(checks::self_version::Chk::new()) as Box<dyn Check>];

    while !new_checks.is_empty() {
        // Go to the “before” folder
        std::env::set_current_dir(&base_path).with_context(|| {
            format!(
                "switching the current directory to base worktree {:?}",
                base_path,
            )
        })?;

        // Run the “before” tests
        for c in new_checks.iter_mut() {
            c.run_before();
        }

        // Go to the “after” folder
        std::env::set_current_dir(&to_check_path).with_context(|| {
            format!(
                "switching the current directory to to-check worktree {:?}",
                to_check_path,
            )
        })?;

        // Run the “after” tests
        for c in new_checks.iter_mut() {
            c.run_after();
        }

        // Update our check list
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

    // Go back to the initial folder
    std::env::set_current_dir(&initial_cwd).with_context(|| {
        format!(
            "switching the current directory back to initial folder {:?}",
            initial_cwd,
        )
    })?;

    // Clean up the worktrees
    std::mem::drop(tempdir);
    println!("{}: pruning the no-longer-existing worktrees", info());
    let worktrees = repo.worktrees().context("listing the worktrees")?;
    for worktree in &worktrees {
        if let Some(wname) = worktree {
            let w = repo
                .find_worktree(wname)
                .with_context(|| format!("opening worktree {}", wname))?;
            if w.is_prunable(None)
                .with_context(|| format!("checking if worktree {} is prunable", wname))?
            {
                w.prune(None)
                    .with_context(|| format!("pruning worktree {}", wname))?;
            }
        }
    }

    // Display the report
    println!();
    println!();
    println!("Report to be pasted in the PR message");
    println!("-------------------------------------");
    println!();
    println!("##### nixpkgs-check report");
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
