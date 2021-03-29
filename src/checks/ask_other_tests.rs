use anyhow::Context;
use crossbeam_channel::Receiver;

pub struct Chk {
    tests: Vec<String>,
}

impl Chk {
    pub fn new() -> anyhow::Result<Chk> {
        let mut tests = Vec::new();

        let choices = vec!["NixOS", "MacOS", "Other Linux distributions"];
        let chosen: Vec<usize> = dialoguer::MultiSelect::with_theme(&*crate::theme())
            .with_prompt("on what machines did you already attempt building? [space to select, enter to validate]")
            .items(&choices)
            .interact()
            .context("asking the user on which machine they already attempted building")?;
        for (i, c) in choices.iter().enumerate() {
            tests.push(match chosen.contains(&i) {
                true => format!("✔ built on {}", c),
                false => format!("😢 not built on {}", c),
            });
        }

        loop {
            let test: String = dialoguer::Input::with_theme(&*crate::theme())
                .allow_empty(true)
                .with_prompt("what other manual tests did you run? [empty to stop]")
                .interact_text()
                .context("asking the user for other tests")?;
            if test.len() == 0 {
                break;
            }
            tests.push(test);
        }

        Ok(Chk { tests })
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid(uuid::Uuid::from_u128(0xddcd70b462d6ff6d937188828a142d7b))
    }

    fn name(&self) -> String {
        "ask-other-tests".to_string()
    }

    fn run_before(&mut self, _: &Receiver<()>) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn run_after(&mut self, _: &Receiver<()>) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn additional_needed_tests(&self) -> anyhow::Result<Vec<Box<dyn crate::Check>>> {
        Ok(vec![])
    }

    fn report(&self) -> String {
        let mut res = String::from("**manual tests declared performed:**\n");
        for test in &self.tests {
            res += &format!(" * {}\n", test);
        }
        res
    }
}
