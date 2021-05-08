use anyhow::{anyhow, Context};
use crossbeam_channel::Receiver;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    rc::Rc,
};

pub struct Chk {
    pkg: String,
    outs_dir: Rc<tempfile::TempDir>,

    new_bins: HashMap<String, Option<bool>>,
    updated_bins: HashMap<String, Option<(bool, bool)>>,
    removed_bins: HashSet<String>,
}

impl Chk {
    pub fn new(pkg: String, outs_dir: Rc<tempfile::TempDir>) -> Chk {
        Chk {
            pkg,
            outs_dir,
            new_bins: HashMap::new(),
            updated_bins: HashMap::new(),
            removed_bins: HashSet::new(),
        }
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid_param(
            uuid::Uuid::from_u128(0x72a00d3835657d0f526fac33a9764ba8),
            &self.pkg,
        )
    }

    fn name(&self) -> String {
        format!("run-binaries({})", self.pkg)
    }

    fn run_before(&mut self, _: &Receiver<()>) -> anyhow::Result<()> {
        Ok(())
    }

    fn run_after(&mut self, killer: &Receiver<()>) -> anyhow::Result<()> {
        // List the binaries
        let bin_dir = match std::fs::read_dir(self.outs_dir.path().join("base").join("bin")) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e).context(format!("listing the base binaries for {}", self.pkg)),
        };
        let base_bins = bin_dir
            .collect::<std::io::Result<Vec<_>>>()
            .with_context(|| format!("listing the base binaries for {}", self.pkg))?
            .into_iter()
            .map(|f| f.file_name().to_str().map(|s| s.to_string()))
            .collect::<Option<Vec<String>>>()
            .ok_or_else(|| anyhow!("a base binary for {} had a non-utf8 name"))?
            .into_iter()
            .filter(|f| !(f.starts_with(".") && f.ends_with("-wrapped")))
            .collect::<HashSet<String>>();
        let to_check_bins = std::fs::read_dir(self.outs_dir.path().join("to-check").join("bin"))
            .with_context(|| format!("listing the to-check binaries for {}", self.pkg))?
            .collect::<std::io::Result<Vec<_>>>()
            .with_context(|| format!("listing the to-check binaries for {}", self.pkg))?
            .into_iter()
            .map(|f| f.file_name().to_str().map(|s| s.to_string()))
            .collect::<Option<Vec<String>>>()
            .ok_or_else(|| anyhow!("a to-check binary for {} had a non-utf8 name"))?
            .into_iter()
            .filter(|f| !(f.starts_with(".") && f.ends_with("-wrapped")))
            .collect::<HashSet<String>>();

        // Figure out which binaries to run
        let choices = to_check_bins.iter().cloned().collect::<Vec<_>>();
        let chosen: Vec<usize> = dialoguer::MultiSelect::with_theme(&*crate::theme())
            .with_prompt(&format!(
                "which binaries of package {} do you want to run?",
                self.pkg
            ))
            .items(&choices)
            .interact()
            .context("asking the user which binaries they want to run")?;
        let chosen: HashMap<String, bool> = choices
            .into_iter()
            .enumerate()
            .map(|(i, c)| (c, chosen.contains(&i)))
            .collect();

        // Run and fill in the results
        self.removed_bins = base_bins.difference(&to_check_bins).cloned().collect();
        self.new_bins = to_check_bins
            .difference(&base_bins)
            .map(|bin| {
                Ok((
                    bin.clone(),
                    if chosen[bin] {
                        Some(run_binary(
                            killer,
                            self.outs_dir.path(),
                            "base",
                            &bin,
                            &self.pkg,
                        )?)
                    } else {
                        None
                    },
                ))
            })
            .collect::<anyhow::Result<_>>()?;
        self.updated_bins = to_check_bins
            .intersection(&base_bins)
            .map(|bin| {
                Ok((
                    bin.clone(),
                    if chosen[bin] {
                        Some((
                            run_binary(killer, self.outs_dir.path(), "base", &bin, &self.pkg)?,
                            run_binary(killer, self.outs_dir.path(), "to-check", &bin, &self.pkg)?,
                        ))
                    } else {
                        None
                    },
                ))
            })
            .collect::<anyhow::Result<_>>()?;

        Ok(())
    }

    fn additional_needed_tests(&self) -> anyhow::Result<Vec<Box<dyn crate::Check>>> {
        Ok(Vec::new())
    }

    fn report(&self) -> String {
        if self.new_bins.is_empty() && self.updated_bins.is_empty() && self.removed_bins.is_empty()
        {
            return String::new();
        }
        let mut res = format!("**binaries of {}:**\n", self.pkg);
        if !self.removed_bins.is_empty() {
            res += &format!("  * *removed binaries:* 😢 {:?}\n", self.removed_bins);
        }
        if !self.new_bins.is_empty() {
            res += "  * *added binaries:*\n";
            for (bin, test) in &self.new_bins {
                match test {
                    None => res += &format!("    * 😢 {} was not run\n", bin),
                    Some(true) => res += &format!("    * ✔ {} was run successfully\n", bin),
                    Some(false) => res += &format!("    * 😢 {} was run unsuccessfully\n", bin),
                }
            }
            res += "\n";
        }
        if !self.updated_bins.is_empty() {
            res += "  * *updated binaries:*\n";
            for (bin, test) in &self.updated_bins {
                match test {
                    None => res += &format!("    * 😢 {} was not run\n", bin),
                    Some((true, true)) => {
                        res += &format!("    * ✔ {} continued running successfully\n", bin)
                    }
                    Some((true, false)) => res += &format!("    * ❌ {} started failing\n", bin),
                    Some((false, true)) => {
                        res += &format!("    * 💚 {} started running successfully again\n", bin)
                    }
                    Some((false, false)) => res += &format!("    * 😢 {} still fails\n", bin),
                }
            }
            res += "\n";
        }
        res
    }
}

/// Returns true iff the binary was run successfully
fn run_binary(
    killer: &Receiver<()>,
    outs_dir: &Path,
    version: &str,
    bin: &str,
    pkg: &str,
) -> anyhow::Result<bool> {
    let theme = crate::theme();
    let mut try_arguments = vec!["--version", "--help", "-h", "-V"].into_iter().fuse();
    loop {
        // Ask the user which arguments to pass
        let mut args = dialoguer::Input::with_theme(&*theme);
        args.with_prompt(format!(
            "what arguments should we pass to the {} version of {} in package {}?",
            version, bin, pkg
        ))
        .allow_empty(true);
        if let Some(arg) = try_arguments.next() {
            args.with_initial_text(arg);
        }
        let args: String = args
            .interact_text()
            .context("asking the user what parameters to pass")?;
        let args = args.split(" ").collect::<Vec<_>>();

        // Run the binary
        crate::run(
            killer,
            false,
            &outs_dir.join(version).join("bin").join(bin),
            &args,
        )
        .with_context(|| format!("running binary {} of package {}", bin, pkg))?;

        // Ask the user whether the run was a success
        let res = dialoguer::Select::with_theme(&*theme)
            .with_prompt("did the binary work?")
            .items(&["Yes", "No", "Try again with other arguments"])
            .default(2)
            .interact()
            .context("asking the user whether the binary worked")?;

        // And return if we can
        match res {
            0 => return Ok(true),
            1 => return Ok(false),
            _ => continue,
        }
    }
}
