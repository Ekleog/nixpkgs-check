use anyhow::{anyhow, bail, Context};

pub struct Chk {
    sandboxing: bool,
}

impl Chk {
    pub fn new() -> anyhow::Result<Chk> {
        let sandboxing = match crate::nix(&["show-config", "--json"])
            .context("reading nix's config")?
            .get("sandbox")
            .ok_or_else(|| anyhow!("nix show-config does not list the sandboxing state"))?
            .get("value")
            .ok_or_else(|| {
                anyhow!("nix show-config does not give the value of the sandboxing state")
            })?
            .as_str()
            .ok_or_else(|| anyhow!("nix show-config's sandboxing state is not a string"))?
        {
            "true" => true,
            "false" => false,
            _ => bail!("nix show-config's sandboxing state is neither true nor false"),
        };
        Ok(Chk { sandboxing })
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid(uuid::Uuid::from_u128(0xd203e52c069ece82dde3c43cf82723f8))
    }

    fn name(&self) -> String {
        "environment".to_string()
    }

    fn run_before(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn run_after(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn additional_needed_tests(&self) -> Vec<Box<dyn crate::Check>> {
        vec![]
    }

    fn report(&self) -> String {
        format!(
            "**version:** `{} v{}` on {} {} sandboxing",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            detect_environment(),
            if self.sandboxing { "with" } else { "without" },
        )
    }
}

fn detect_environment() -> String {
    // TODO: upstream auto-detection to os_type?
    if std::path::Path::new("/etc/NIXOS").exists() {
        format!(
            "NixOS {}",
            std::fs::read_to_string("/run/current-system/nixos-version")
                .unwrap_or(String::from(""))
        )
    } else {
        let info = os_type::current_platform();
        format!("{:?} {}", info.os_type, info.version)
    }
}