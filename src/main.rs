use anyhow::{anyhow, Context};
use nixpkgs_check::{checks, Check, State};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

const STATE_FILE: &str = "state.json";

fn error() -> console::StyledObject<&'static str> {
    console::style("error").red().bold()
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

    let (killer_s, killer_r) = crossbeam_channel::bounded::<()>(0);
    ctrlc::set_handler(move || {
        let _ = killer_s.try_send(());
    })
    .context("setting ctrl-c handler")?;

    let (checkout_base_done_s, checkout_base_done_r) =
        std::sync::mpsc::channel::<anyhow::Result<()>>();
    let (checkout_tocheck_done_s, checkout_tocheck_done_r) =
        std::sync::mpsc::channel::<anyhow::Result<()>>();
    {
        let repo_path = opt.repo_path.clone();
        let base_path = base_path.clone();
        let to_check_path = to_check_path.clone();
        let base_ref = opt.base_ref.clone();
        let to_check_ref = opt.to_check_ref.clone();
        std::thread::spawn(move || {
            let (repo, base_oid, to_check_oid) =
                match prepare_checking_out(&repo_path, &base_ref, &to_check_ref) {
                    Ok(res) => res,
                    Err(e) => {
                        let _ = checkout_base_done_s.send(Err(e));
                        return;
                    }
                };
            match setup_checkout(&repo, &base_path, base_oid) {
                Ok(()) => {
                    let _ = checkout_base_done_s.send(Ok(()));
                }
                Err(e) => {
                    let _ = checkout_base_done_s.send(Err(e));
                    return;
                }
            }
            let _ = checkout_tocheck_done_s.send(
                setup_checkout(&repo, &to_check_path, to_check_oid)
                    .context("checking out to-check worktree"),
            );
        });
    }

    let xdg_dirs = xdg::BaseDirectories::with_prefix("nixpkgs-check")
        .context("finding the right XDG directories")?;
    let mut state = match xdg_dirs.find_data_file(STATE_FILE) {
        Some(path) => State::load(
            std::fs::File::open(&path).with_context(|| format!("opening state file {:?}", path))?,
        )
        .with_context(|| format!("parsing state file {:?}", path))?,
        None => State::default(),
    };

    let changed_pkgs = autodetect_changed_pkgs(&opt.repo_path, &opt.base_ref, &opt.to_check_ref)
        .context("auto-detecting which packages were changed based on commit message")?;

    // Note: these three checks all don't have the run_{before,after}
    // methods implemented
    let mut checks = vec![
        Box::new(checks::environment::Chk::new(&killer_r).context("checking the environment")?)
            as Box<dyn Check>,
        Box::new(checks::ask_pkg_names::Chk::new(changed_pkgs)?),
        Box::new(checks::ask_other_tests::Chk::new()?),
        Box::new(checks::confirm_contributing::Chk::new(&mut state)?),
    ];
    let mut new_checks = checks
        .iter()
        .map(|c| c.additional_needed_tests())
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flat_map(|c| c.into_iter())
        .collect::<Vec<_>>();

    match checkout_base_done_r.try_recv() {
        Ok(r) => r,
        Err(_) => {
            println!("you answered the questions too fast, we're still checking out the base worktree, please wait…");
            checkout_base_done_r
                .recv()
                .context("receiving base checkout result")?
        }
    }
    .context("checking out base worktree")?;

    let mut is_first_run = true; // used to know whether to wait for checkout_tocheck_done_r
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
            println!("running base version of {}", c.name());
            c.run_before(&killer_r)
                .with_context(|| format!("running check {} on base version", c.name()))?;
        }

        // If this is our first run, the to-check worktree may not be
        // ready yet, in which case let's wait for it.
        if is_first_run {
            is_first_run = false;
            match checkout_tocheck_done_r.try_recv() {
                Ok(r) => r,
                Err(_) => {
                    println!("the builds completed too fast, we're still checking out the to-check worktree, please wait…");
                    checkout_tocheck_done_r
                        .recv()
                        .context("receiving to-check checkout result")?
                }
            }
            .context("checking out base worktree")?;
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
            println!("running to-check version of {}", c.name());
            c.run_after(&killer_r)
                .with_context(|| format!("running check {} on to-check version", c.name()))?;
        }

        // Update our check list
        let new_new_checks = new_checks
            .iter()
            .map(|c| c.additional_needed_tests())
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|c| c.into_iter())
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
    println!("pruning the no-longer-existing worktrees");
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

    // Save the state
    let state_file = xdg_dirs
        .place_data_file(STATE_FILE)
        .context("creating the directories for the state file")?;
    state
        .save(
            std::fs::File::create(&state_file)
                .with_context(|| format!("creating state file {:?}", state_file))?,
        )
        .with_context(|| format!("saving the state to state file {:?}", state_file))?;

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
                console::style(format!(
                    "{}",
                    errs.next().expect("got error chain with zero errors")
                ))
                .bold()
            );
            for e in errs {
                eprintln!("  while {}", console::style(format!("{}", e)).bold());
            }
            std::process::exit(1);
        }
    }
}

/// Returns (repo, base-oid, to-check-oid) on success
fn prepare_checking_out(
    repo_path: &Path,
    base_ref: &str,
    to_check_ref: &str,
) -> anyhow::Result<(git2::Repository, git2::Oid, git2::Oid)> {
    // Open the repo
    let repo = git2::Repository::open(repo_path)
        .with_context(|| format!("opening the nixpkgs repo {:?}", repo_path))?;

    // Resolve the to-check reference name
    let to_check_oid = repo
        .revparse_single(to_check_ref)
        .with_context(|| {
            format!(
                "finding reference {:?} in repo {:?}",
                to_check_ref, repo_path
            )
        })?
        .id();

    // The base reference is actually merge-base(base, to-check)
    let base_oid = {
        let base_obj = repo
            .revparse_single(&base_ref)
            .with_context(|| format!("finding reference {:?} in repo {:?}", base_ref, repo_path))?;
        repo.merge_base(base_obj.id(), to_check_oid)
            .context("finding the merge-base of the base reference and the to-check reference")?
    };

    Ok((repo, base_oid, to_check_oid))
}

fn setup_checkout(repo: &git2::Repository, path: &Path, oid: git2::Oid) -> anyhow::Result<()> {
    let wt = repo
        .worktree(
            uuid::Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer()),
            &path,
            None,
        )
        .context("creating worktree")?;
    let wt_repo =
        git2::Repository::open_from_worktree(&wt).context("opening worktree as a repository")?;
    wt_repo
        .checkout_tree(
            &wt_repo
                .find_object(oid, None)
                .context("converting repo object to worktree object")?,
            None,
        )
        .with_context(|| format!("checking out commit {} in worktree at path {:?}", oid, path))?;

    Ok(())
}

fn autodetect_changed_pkgs(
    repo_path: &Path,
    base_ref: &str,
    to_check_ref: &str,
) -> anyhow::Result<HashSet<String>> {
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

    let mut pkgs = HashSet::new();
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
            pkgs.insert(pkg.to_string());
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
