use anyhow::Context;

pub struct Chk {
    tests: Vec<String>,
}

impl Chk {
    pub fn new() -> anyhow::Result<Chk> {
        let mut tests = Vec::new();
        loop {
            let test: String =
                dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .allow_empty(true)
                    .with_prompt("did you run manual tests? [empty to stop]")
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
        let mut res = String::from("**manual tests declared performed:**");
        for test in &self.tests {
            res += &format!("\n * {}", test);
        }
        res
    }
}
