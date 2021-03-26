pub struct Chk(());

impl Chk {
    pub fn new() -> Chk {
        Chk(())
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid(uuid::Uuid::from_u128(0xd203e52c069ece82dde3c43cf82723f8))
    }

    fn name(&self) -> String {
        "check-self-version".to_string()
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
            "**version:** `{} v{}` on {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            detect_environment(),
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
