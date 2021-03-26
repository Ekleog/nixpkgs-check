use anyhow::{anyhow, Context};
use std::convert::TryFrom;

pub struct Chk {
    pkg: String,
    closure_size_before: Option<bytesize::ByteSize>,
    closure_size_after: Option<bytesize::ByteSize>,
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
        let cs_before = self.closure_size_before.unwrap_or_else(|| {
            panic!(
                "did not check closure size for the base version of {}",
                self.pkg
            )
        });
        let cs_after = self.closure_size_after.unwrap_or_else(|| {
            panic!(
                "did not check closure size for the base version of {}",
                self.pkg
            )
        });
        let cs_before_i = i64::try_from(cs_before.as_u64()).unwrap();
        let cs_after_i = i64::try_from(cs_after.as_u64()).unwrap();
        let diff: i64 = cs_after_i - cs_before_i;
        let emoji = match diff {
            _ if diff.abs() < cs_before_i / 10 => "✔",
            _ if diff > 0 => "💚",
            _ => "😢",
        };
        let did_incr = match diff {
            _ if diff > 0 => "increased",
            _ => "decreased",
        };
        let abs_diff = bytesize::ByteSize::b(diff.abs() as u64);
        format!(
            "**path-info for {}:** {} {} by {}, from {} to {}",
            self.pkg, emoji, did_incr, abs_diff, cs_before, cs_after
        )
    }
}

fn closure_size(version: &str, pkg: &str) -> anyhow::Result<bytesize::ByteSize> {
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
    Ok(bytesize::ByteSize::b(
        res.get(0)
            .ok_or_else(|| anyhow!("output of nix path-info -S is not an array"))?
            .get("closureSize")
            .ok_or_else(|| anyhow!("output of nix path-info -S does not have closureSize element"))?
            .as_u64()
            .ok_or_else(|| anyhow!("output of nix path-info -S has a non-integer closureSize"))?,
    ))
}
