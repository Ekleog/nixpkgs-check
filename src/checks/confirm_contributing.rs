use anyhow::Context;
use console::style;
use crossbeam_channel::Receiver;

const CONTRIBUTING: &str = include_str!(env!("CONTRIBUTING_MD_PATH"));

pub struct Chk {
    confirmed: bool,
}

impl Chk {
    pub fn new(state: &mut crate::State) -> anyhow::Result<Chk> {
        let last_contributing = state.last_contributing.as_ref().map(|c| c as &str);
        if last_contributing != Some(CONTRIBUTING) {
            println!(
                "{}",
                style("CONTRIBUTING.md changed since you last read it").bold()
            );
            println!("--------------------");
            for l in diff::lines(last_contributing.unwrap_or(""), CONTRIBUTING) {
                match l {
                    diff::Result::Left(l) => println!("{}", style(format!("-{}", l)).red()),
                    diff::Result::Right(l) => println!("{}", style(format!("+{}", l)).green()),
                    diff::Result::Both(l, _) => println!(" {}", l),
                }
            }
            println!("--------------------");
            loop {
                let read_it = dialoguer::Confirm::with_theme(&*crate::theme())
                    .with_prompt("did you read the changes?")
                    .interact()
                    .context("asking the user for reading the contributing.md changes")?;
                if read_it {
                    state.last_contributing = Some(CONTRIBUTING.to_string());
                    break;
                } else {
                    println!("please read the changes to CONTRIBUTING.md to proceed");
                }
            }
        }
        let confirmed = dialoguer::Confirm::with_theme(&*crate::theme())
            .with_prompt("do the changes respect the rules of CONTRIBUTING.md?")
            .interact()
            .context("asking the user whether the changes respect CONTRIBUTING.md")?;
        Ok(Chk { confirmed })
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid(uuid::Uuid::from_u128(0x5281f549224719a2b6104c3a5fbc9c12))
    }

    fn name(&self) -> String {
        "confirm-contributing".to_string()
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
        format!(
            "**complies with contributing.md:** {}",
            if self.confirmed { "✔ yes" } else { "😢 no" },
        )
    }
}
