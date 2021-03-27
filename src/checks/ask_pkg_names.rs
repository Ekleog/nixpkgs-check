use ansi_term::Style;
use anyhow::Context;

pub struct Chk {
    pkgs: Vec<String>,
}

impl Chk {
    pub fn new(mut pkgs: Vec<String>) -> anyhow::Result<Chk> {
        println!(
            "{} {:?}",
            Style::new().bold().paint("Autodetected changed packages:"),
            pkgs
        );
        loop {
            let pkg: String = dialoguer::Input::with_theme(&crate::theme())
                .allow_empty(true)
                .with_prompt("do you want to test other attribute paths? [empty to stop]")
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
        crate::CheckId::from_uuid(uuid::Uuid::from_u128(0xd173872f19b0a1d30b96ed9929e23250))
    }

    fn name(&self) -> String {
        "ask-package-names".to_string()
    }

    fn run_before(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn run_after(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn additional_needed_tests(&self) -> Vec<Box<dyn crate::Check>> {
        self.pkgs
            .iter()
            .map(|pkg| {
                Box::new(crate::checks::build::Chk::new(pkg.clone())) as Box<dyn crate::Check>
            })
            .collect()
    }

    fn report(&self) -> String {
        format!("**packages declared changed:** {:?}", self.pkgs)
    }
}
