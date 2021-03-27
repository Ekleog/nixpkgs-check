use ansi_term::{Colour::Red, Style};
use anyhow::{anyhow, Context};
use nixpkgs_check::{checks, Check};
use std::path::{Path, PathBuf};
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
    #[structopt(long, short, default_value = "master")]
    base_ref: String,

    /// The path towards the nixpkgs repository
    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

fn run(opt: Opt) -> anyhow::Result<()> {
    // Checkout the commits in worktrees
    let tempdir = tempfile::tempdir().context("creating temporary directory")?;
    let base_path = tempdir.path().join("base");
    let to_check_path = tempdir.path().join("to-check");
    let initial_cwd = std::env::current_dir().context("recovering current working directory")?;

    let (checkout_done_s, checkout_done_r) = std::sync::mpsc::channel::<anyhow::Result<()>>();
    {
        let repo_path = opt.repo_path.clone();
        let base_path = base_path.clone();
        let to_check_path = to_check_path.clone();
        let base_ref = opt.base_ref.clone();
        let to_check_ref = opt.to_check_ref.clone();
        std::thread::spawn(move || {
            checkout_done_s
                .send(setup_checkouts(
                    &repo_path,
                    &base_path,
                    &to_check_path,
                    &base_ref,
                    &to_check_ref,
                ))
                .expect("failed sending checkout result");
        });
    }

    let changed_pkgs = autodetect_changed_pkgs(&opt.repo_path, &opt.base_ref, &opt.to_check_ref)
        .context("auto-detecting which packages were changed based on commit message")?;

    let mut checks = vec![];
    let mut new_checks = vec![
        Box::new(checks::environment::Chk::new().context("checking the environment")?)
            as Box<dyn Check>,
        Box::new(checks::ask_pkg_names::Chk::new(changed_pkgs)?),
        Box::new(checks::ask_other_tests::Chk::new()?),
    ];

    match checkout_done_r.try_recv() {
        Ok(r) => r,
        Err(_) => {
            println!("Currently checking out the worktrees, please wait...");
            checkout_done_r
                .recv()
                .context("receiving checkout result")?
        }
    }
    .context("checking out worktrees")?;

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
            println!("Running base version of {}", c.name());
            c.run_before()
                .with_context(|| format!("running check {} on base version", c.name()))?;
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
            println!("Running to-check version of {}", c.name());
            c.run_after()
                .with_context(|| format!("running check {} on to-check version", c.name()))?;
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
    let repo = git2::Repository::open(&opt.repo_path)
        .with_context(|| format!("opening the nixpkgs repo {:?}", &opt.repo_path))?;
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
    println!("### nixpkgs-check report");
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

fn setup_checkouts(
    repo_path: &Path,
    base_path: &Path,
    to_check_path: &Path,
    base_ref: &str,
    to_check_ref: &str,
) -> anyhow::Result<()> {
    // Open the repo
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("opening the nixpkgs repo {:?}", repo_path))?;

    // Resolve the reference names
    let base_obj = repo
        .revparse_single(&base_ref)
        .with_context(|| format!("finding reference {:?} in repo {:?}", base_ref, repo_path))?;
    let to_check_obj = repo.revparse_single(to_check_ref).with_context(|| {
        format!(
            "finding reference {:?} in repo {:?}",
            to_check_ref, repo_path
        )
    })?;

    // The base reference is actually merge-base(base, to-check)
    let base_oid = repo
        .merge_base(base_obj.id(), to_check_obj.id())
        .context("finding the merge-base of the base reference and the to-check reference")?;

    // Checkout the worktrees
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
                .find_object(base_oid, None)
                .context("converting repo object to worktree object")?,
            None,
        )
        .context("checking out base commit in worktree")?;

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

    Ok(())
}

fn autodetect_changed_pkgs(
    repo_path: &Path,
    base_ref: &str,
    to_check_ref: &str,
) -> anyhow::Result<Vec<String>> {
    // Open the repo
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("opening the nixpkgs repo {:?}", repo_path))?;

    // Resolve the reference names
    let base_obj = repo
        .revparse_single(&base_ref)
        .with_context(|| format!("finding reference {:?} in repo {:?}", base_ref, repo_path))?;
    let to_check_obj = repo.revparse_single(to_check_ref).with_context(|| {
        format!(
            "finding reference {:?} in repo {:?}",
            to_check_ref, repo_path
        )
    })?;

    // The base reference is actually merge-base(base, to-check)
    let base_oid = repo
        .merge_base(base_obj.id(), to_check_obj.id())
        .context("finding the merge-base of the base reference and the to-check reference")?;

    let mut pkgs = Vec::new();
    let mut commit = to_check_obj
        .peel_to_commit()
        .context("peeling to-check object to a commit")?;
    loop {
        if commit.id() == base_oid {
            break;
        }
        if commit.parent_count() == 1 {
            // merge commits are usually not commits we're interested in
            let summary = commit
                .summary()
                .ok_or_else(|| anyhow!("commit {} has a non-utf-8 summary", commit.id()))?;
            let pkg = summary.split(":").next().ok_or_else(|| {
                anyhow!(
                    "commit {} has summary that does not respect the convention: {}",
                    commit.id(),
                    summary
                )
            })?;
            pkgs.push(pkg.to_string());
        }
        commit = commit
            .parent(0)
            .with_context(|| format!("recovering parent of commit {}", commit.id()))?;
        // TODO: we should probably do a BFS or similar for more
        // complex PR histories, but this should be enough for a first
        // version
    }

    Ok(pkgs)
}
