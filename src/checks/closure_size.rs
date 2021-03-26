use anyhow::{anyhow, Context};

pub struct Chk {
    pkg: String,
    closure_size_before: Option<u64>,
    closure_size_after: Option<u64>,
}

impl Chk {
    pub fn new(pkg: String) -> Chk {
        Chk {
            pkg,
            closure_size_before: None,
            closure_size_after: None,
        }
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid_param(
            uuid::Uuid::from_u128(0x4949d1064292d3b9f7048307b0e9ce7),
            &self.pkg,
        )
    }

    fn name(&self) -> String {
        format!("closure-size({})", self.pkg)
    }

    fn run_before(&mut self) -> anyhow::Result<()> {
        self.closure_size_before = Some(closure_size("base", &self.pkg)?);
        Ok(())
    }

    fn run_after(&mut self) -> anyhow::Result<()> {
        self.closure_size_after = Some(closure_size("to-check", &self.pkg)?);
        Ok(())
    }

    fn additional_needed_tests(&self) -> Vec<Box<dyn crate::Check>> {
        vec![]
    }

    fn report(&self) -> String {
        let closure_size_before = self.closure_size_before.unwrap_or_else(|| {
            panic!(
                "did not check closure size for the base version of {}",
                self.pkg
            )
        });
        let closure_size_after = self.closure_size_after.unwrap_or_else(|| {
            panic!(
                "did not check closure size for the base version of {}",
                self.pkg
            )
        });
        format!(
            "**path-info for {}:** went from {} to {}",
            self.pkg, closure_size_before, closure_size_after
        )
    }
}

fn closure_size(version: &str, pkg: &str) -> anyhow::Result<u64> {
    let out = std::process::Command::new("nix")
        .args(&["path-info", "--json", "-S", &crate::nix_eval_for(pkg)])
        .stderr(std::process::Stdio::inherit())
        .output()
        .with_context(|| {
            format!(
                "getting the closure size of the {} version of package {}",
                version, pkg
            )
        })?
        .stdout;
    let res = serde_json::from_slice::<serde_json::Value>(&out)
        .context("parsing the output of `nix path-info --json -S` as JSON")?;
    Ok(res
        .get(0)
        .ok_or_else(|| anyhow!("output of nix path-info -S is not an array"))?
        .get("closureSize")
        .ok_or_else(|| anyhow!("output of nix path-info -S does not have closureSize element"))?
        .as_u64()
        .ok_or_else(|| anyhow!("output of nix path-info -S has a non-integer closureSize"))?)
}
