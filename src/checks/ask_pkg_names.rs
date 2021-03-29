use anyhow::Context;
use crossbeam_channel::Receiver;
use std::collections::HashSet;

pub struct Chk {
    pkgs: HashSet<String>,
}

impl Chk {
    pub fn new(pkgs: HashSet<String>) -> anyhow::Result<Chk> {
        let choices = pkgs.into_iter().collect::<Vec<_>>();
        let chosen = dialoguer::MultiSelect::with_theme(&crate::theme())
            .with_prompt("which packages do you want to test? [space to select, enter to validate]")
            .items(&choices)
            .defaults(&choices.iter().map(|_| true).collect::<Vec<_>>())
            .interact()
            .context("asking the user for package names")?;

        let mut pkgs = chosen
            .into_iter()
            .map(|i| choices[i].clone())
            .collect::<HashSet<String>>();
        loop {
            let pkg: String = dialoguer::Input::with_theme(&crate::theme())
                .allow_empty(true)
                .with_prompt("what other packages do you want to test? [empty to stop]")
                .interact_text()
                .context("asking the user for package names")?;
            if pkg.len() == 0 {
                break;
            }
            pkgs.insert(pkg);
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

    fn run_before(&mut self, _: &Receiver<()>) -> anyhow::Result<()> {
        Ok(())
    }

    fn run_after(&mut self, _: &Receiver<()>) -> anyhow::Result<()> {
        Ok(())
    }

    fn additional_needed_tests(&self) -> anyhow::Result<Vec<Box<dyn crate::Check>>> {
        self.pkgs
            .iter()
            .map(|pkg| {
                Ok(Box::new(crate::checks::build::Chk::new(pkg.clone())?) as Box<dyn crate::Check>)
            })
            .collect()
    }

    fn report(&self) -> String {
        format!("**packages declared changed:** {:?}", self.pkgs)
    }
}
