use anyhow::Context;
use crossbeam_channel::Receiver;
use std::path::Path;

pub mod checks;

#[derive(PartialEq, Eq)]
pub struct CheckId(String);

impl CheckId {
    fn from_uuid(uuid: uuid::Uuid) -> CheckId {
        CheckId(
            uuid.to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer())
                .to_string(),
        )
    }

    fn from_uuid_param(uuid: uuid::Uuid, param: &str) -> CheckId {
        CheckId(
            uuid.to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer())
                .to_string()
                + "-"
                + param,
        )
    }
}

pub trait Check {
    /// A UUID for this check (including any dynamic parameters it
    /// could have that might make it different from other checks of
    /// the same type)
    fn uuid(&self) -> CheckId;

    /// The human-meaningful name for this check
    fn name(&self) -> String;

    /// This is run on the checkout before the changes
    fn run_before(&mut self, killer: &Receiver<()>) -> anyhow::Result<()>;

    /// This is run on the checkout after the changes
    fn run_after(&mut self, killer: &Receiver<()>) -> anyhow::Result<()>;

    /// Returns the tests that are additionally needed
    fn additional_needed_tests(&self) -> anyhow::Result<Vec<Box<dyn Check>>>;

    /// Generate the report
    fn report(&self) -> String;
}

fn nixpkgs() -> &'static str {
    "(import ./. { overlays = []; })"
}

fn nix_eval_for(pkg: &str) -> String {
    format!("({}.{})", nixpkgs(), pkg)
}

fn nix(killer: &Receiver<()>, args: &[&str]) -> anyhow::Result<Option<serde_json::Value>> {
    run_nix(killer, true, args)?
        .map(|out| {
            serde_json::from_slice(&out.stdout).context("parsing the output of the nix command")
        })
        .transpose()
}

fn run_nix(
    killer: &Receiver<()>,
    capture_stdout: bool,
    args: &[&str],
) -> anyhow::Result<Option<std::process::Output>> {
    run(killer, capture_stdout, Path::new("nix"), args)
}

fn run(
    killer: &Receiver<()>,
    capture_stdout: bool,
    path: &Path,
    args: &[&str],
) -> anyhow::Result<Option<std::process::Output>> {
    let mut process = std::process::Command::new(path);
    process.args(args).stderr(std::process::Stdio::inherit());
    if capture_stdout {
        process.stdout(std::process::Stdio::piped());
    } else {
        process.stdout(std::process::Stdio::inherit());
    }
    let mut child = process.spawn().context("spawning the nix command")?;
    while child.try_wait().context("waiting for nix")?.is_none() {
        if let Ok(()) = killer.recv_timeout(std::time::Duration::from_millis(50)) {
            // Leave some time for the ctrl-c to reach nix
            std::thread::sleep(std::time::Duration::from_millis(200));
            let _ = child.kill();
            return Ok(None);
        }
    }
    Ok(Some(child.wait_with_output().context(
        "retrieving the output from a known-completed process",
    )?))
}

fn theme() -> impl dialoguer::theme::Theme {
    dialoguer::theme::ColorfulTheme::default()
}
