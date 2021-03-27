use anyhow::Context;

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

    fn run_before(&mut self) -> anyhow::Result<()> {
        self.builds_before = Some(build("base", &self.pkg)?);
        Ok(())
    }

    fn run_after(&mut self) -> anyhow::Result<()> {
        self.builds_after = Some(build("to-check", &self.pkg)?);
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
        let builds_before = self
            .builds_before
            .unwrap_or_else(|| panic!("did not attempt building the base version of {}", self.pkg));
        let builds_after = self.builds_after.unwrap_or_else(|| {
            panic!(
                "did not attempt building the to-check version of {}",
                self.pkg
            )
        });
        match (builds_before, builds_after) {
            (true, true) => format!("**package {}:** ✔ continued building", self.pkg),
            (true, false) => format!("**package {}:** ❌ stopped building", self.pkg),
            (false, true) => format!("**package {}:** 💚 started building again", self.pkg),
            (false, false) => format!("**package {}:** 😢 still does not build", self.pkg),
        }
    }
}

/// Returns true iff the build was successful
fn build(version: &str, pkg: &str) -> anyhow::Result<bool> {
    // TODO: introduce a ctrl-c handler to kill only the nix-build if needed?
    Ok(crate::run_nix(false, &["build", &crate::nix_eval_for(pkg)])
        .with_context(|| format!("building the {} version of package {}", version, pkg))?
        .status
        .success())
}
