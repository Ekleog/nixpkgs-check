use anyhow::{anyhow, Context};
use crossbeam_channel::Receiver;
use std::{path::Path, rc::Rc};

pub struct Chk {
    pkg: String,
    builds_before: Option<bool>,
    builds_after: Option<bool>,
    outs_dir: Rc<tempfile::TempDir>,
}

impl Chk {
    pub fn new(pkg: String) -> anyhow::Result<Chk> {
        Ok(Chk {
            pkg,
            builds_before: None,
            builds_after: None,
            outs_dir: Rc::new(
                tempfile::tempdir()
                    .context("creating temporary directory to hold build results")?,
            ),
        })
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid_param(
            uuid::Uuid::from_u128(0xe2ab3c62ae1f22bd6eee85fb675eaa53),
            &self.pkg,
        )
    }

    fn name(&self) -> String {
        format!("build({})", self.pkg)
    }

    fn run_before(&mut self, killer: &Receiver<()>) -> anyhow::Result<()> {
        self.builds_before = build(killer, self.outs_dir.path(), "base", &self.pkg)?;
        Ok(())
    }

    fn run_after(&mut self, killer: &Receiver<()>) -> anyhow::Result<()> {
        self.builds_after = build(killer, self.outs_dir.path(), "to-check", &self.pkg)?;
        Ok(())
    }

    fn additional_needed_tests(&self) -> anyhow::Result<Vec<Box<dyn crate::Check>>> {
        let mut res = Vec::new();
        if self.builds_before == Some(true) && self.builds_after == Some(true) {
            res.push(
                Box::new(crate::checks::closure_size::Chk::new(self.pkg.clone()))
                    as Box<dyn crate::Check>,
            );
        }
        if self.builds_after == Some(true) {
            res.push(Box::new(crate::checks::run_tests::Chk::new(
                self.pkg.clone(),
            )));
            res.push(Box::new(crate::checks::run_binaries::Chk::new(
                self.pkg.clone(),
                self.outs_dir.clone(),
            )));
        }
        Ok(res)
    }

    fn report(&self) -> String {
        match (self.builds_before, self.builds_after) {
            (None, None) => format!("**package {}:** 😢 both builds interrupted", self.pkg),
            (None, Some(after)) => format!(
                "**package {}:** 😢 base build interrupted, to-check build {}",
                self.pkg,
                if after { "passed" } else { "did not pass" },
            ),
            (Some(before), None) => format!(
                "**package {}:** 😢 to-check build interrupted, base build {}",
                self.pkg,
                if before { "passed" } else { "did not pass" },
            ),
            (Some(true), Some(true)) => format!("**package {}:** ✔ continued building", self.pkg),
            (Some(true), Some(false)) => format!("**package {}:** ❌ stopped building", self.pkg),
            (Some(false), Some(true)) => {
                format!("**package {}:** 💚 started building again", self.pkg)
            }
            (Some(false), Some(false)) => {
                format!("**package {}:** 😢 still does not build", self.pkg)
            }
        }
    }
}

/// Returns true iff the build was successful
fn build(
    killer: &Receiver<()>,
    outs_dir: &Path,
    version: &str,
    pkg: &str,
) -> anyhow::Result<Option<bool>> {
    Ok(crate::run_nix(
        killer,
        false,
        &[
            "build",
            "--out-link",
            outs_dir
                .join(version)
                .to_str()
                .ok_or_else(|| anyhow!("got non-utf8 temporary path"))?,
            &crate::nix_eval_for(pkg),
        ],
    )
    .with_context(|| format!("building the {} version of package {}", version, pkg))?
    .map(|out| out.status.success()))
}
