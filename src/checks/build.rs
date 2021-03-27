use anyhow::Context;
use crossbeam_channel::Receiver;

pub struct Chk {
    pkg: String,
    builds_before: Option<bool>,
    builds_after: Option<bool>,
}

impl Chk {
    pub fn new(pkg: String) -> Chk {
        Chk {
            pkg,
            builds_before: None,
            builds_after: None,
        }
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
        self.builds_before = build(killer, "base", &self.pkg)?;
        Ok(())
    }

    fn run_after(&mut self, killer: &Receiver<()>) -> anyhow::Result<()> {
        self.builds_after = build(killer, "to-check", &self.pkg)?;
        Ok(())
    }

    fn additional_needed_tests(&self) -> Vec<Box<dyn crate::Check>> {
        if self.builds_before == Some(true) && self.builds_after == Some(true) {
            vec![
                Box::new(crate::checks::closure_size::Chk::new(self.pkg.clone()))
                    as Box<dyn crate::Check>,
            ]
        } else {
            vec![]
        }
    }

    fn report(&self) -> String {
        match (self.builds_before, self.builds_after) {
            (None, None) => format!("**package {}:** 😢 both builds interrupted", self.pkg),
            (None, Some(after)) => format!(
                "**package {}:** 😢 initial build interrupted, to-check build {}",
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
fn build(killer: &Receiver<()>, version: &str, pkg: &str) -> anyhow::Result<Option<bool>> {
    // TODO: introduce a ctrl-c handler to kill only the nix-build if needed?
    Ok(
        crate::run_nix(killer, false, &["build", &crate::nix_eval_for(pkg)])
            .with_context(|| format!("building the {} version of package {}", version, pkg))?
            .map(|out| out.status.success()),
    )
}
