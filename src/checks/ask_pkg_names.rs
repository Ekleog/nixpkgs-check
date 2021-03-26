use anyhow::Context;

pub struct Chk {
    pkgs: Vec<String>,
}

impl Chk {
    pub fn new() -> anyhow::Result<Chk> {
        let mut pkgs = Vec::new();
        loop {
            let pkg: String =
                dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .allow_empty(true)
                    .with_prompt("changed package attribute path [empty to stop]")
                    .interact_text()
                    .context("asking the user for package names")?;
            if pkg.len() == 0 {
                break;
            }
            pkgs.push(pkg);
        }
        Ok(Chk { pkgs })
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid(uuid::Uuid::from_u128(0xd203e52c069ece82dde3c43cf82723f8))
    }

    fn run_before(&mut self) {}

    fn run_after(&mut self) {}

    fn additional_needed_tests(&self) -> Vec<Box<dyn crate::Check>> {
        vec![]
    }

    fn report(&self) -> String {
        format!("**packages declared changed:** {:?}", self.pkgs)
    }
}
