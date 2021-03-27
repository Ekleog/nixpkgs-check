use anyhow::Context;

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
    fn run_before(&mut self) -> anyhow::Result<()>;

    /// This is run on the checkout after the changes
    fn run_after(&mut self) -> anyhow::Result<()>;

    /// Returns the tests that are additionally needed
    fn additional_needed_tests(&self) -> Vec<Box<dyn Check>>;

    /// Generate the report
    fn report(&self) -> String;
}

fn nix_eval_for(pkg: &str) -> String {
    format!("((import ./. {{ overlays = []; }}).{})", pkg)
}

fn nix(args: &[&str]) -> anyhow::Result<serde_json::Value> {
    let out = run_nix(true, args)?.stdout;
    serde_json::from_slice(&out).context("parsing the output of the nix command")
}

fn run_nix(capture_stdout: bool, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let mut process = std::process::Command::new("nix");
    process.args(args).stderr(std::process::Stdio::inherit());
    if capture_stdout {
        process.stdout(std::process::Stdio::piped());
    } else {
        process.stdout(std::process::Stdio::inherit());
    }
    process.output().context("executing the nix command")
}

fn theme() -> impl dialoguer::theme::Theme {
    dialoguer::theme::ColorfulTheme::default()
}
