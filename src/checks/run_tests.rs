use anyhow::Context;
use crossbeam_channel::Receiver;
use std::collections::{HashMap, HashSet};

pub struct Chk {
    pkg: String,
    builds_before: HashMap<String, Option<bool>>,
    builds_after: HashMap<String, Option<bool>>,
}

impl Chk {
    pub fn new(pkg: String) -> Chk {
        Chk {
            pkg,
            builds_before: HashMap::new(),
            builds_after: HashMap::new(),
        }
    }
}

impl crate::Check for Chk {
    fn uuid(&self) -> crate::CheckId {
        crate::CheckId::from_uuid_param(
            uuid::Uuid::from_u128(0x4ff16bd3b116c18963d42d2620f0e7f9),
            &self.pkg,
        )
    }

    fn name(&self) -> String {
        format!("run-tests({})", self.pkg)
    }

    fn run_before(&mut self, killer: &Receiver<()>) -> anyhow::Result<()> {
        self.builds_before = build(killer, "base", &self.pkg)?;
        Ok(())
    }

    fn run_after(&mut self, killer: &Receiver<()>) -> anyhow::Result<()> {
        self.builds_after = build(killer, "to-check", &self.pkg)?;
        Ok(())
    }

    fn additional_needed_tests(&self) -> anyhow::Result<Vec<Box<dyn crate::Check>>> {
        Ok(Vec::new())
    }

    fn report(&self) -> String {
        let mut res = format!("**tests of {}:**", self.pkg);
        if self.builds_before.is_empty() && self.builds_after.is_empty() {
            res += " 😢 there are no tests";
            return res;
        }
        res += "\n";

        let tests_before = self
            .builds_before
            .keys()
            .cloned()
            .collect::<HashSet<String>>();
        let tests_after = self
            .builds_after
            .keys()
            .cloned()
            .collect::<HashSet<String>>();

        let removed_tests = tests_before
            .difference(&tests_after)
            .map(|t| (t.clone(), self.builds_before[t]))
            .collect::<HashMap<String, Option<bool>>>();
        let new_tests = tests_after
            .difference(&tests_before)
            .map(|t| (t.clone(), self.builds_after[t]))
            .collect::<HashMap<String, Option<bool>>>();
        let updated_tests = tests_before
            .intersection(&tests_after)
            .map(|t| (t.clone(), (self.builds_before[t], self.builds_after[t])))
            .collect::<HashMap<String, (Option<bool>, Option<bool>)>>();

        if !removed_tests.is_empty() {
            res += &format!("  * *removed tests:* 😢 {:?}\n", removed_tests);
        }
        if !new_tests.is_empty() {
            res += "  * *added tests:*\n";
            for (test, result) in &new_tests {
                match result {
                    None => res += &format!("    * 😢 {} was interrupted\n", test),
                    Some(true) => res += &format!("    * ✔ {} was run successfully\n", test),
                    Some(false) => res += &format!("    * 😢 {} was run unsuccessfully\n", test),
                }
            }
            res += "\n";
        }
        if !updated_tests.is_empty() {
            res += "  * *updated tests:*\n";
            for (test, result) in &updated_tests {
                match result {
                    (None, None) => res += &format!("    * 😢 {} was interrupted twice\n", test),
                    (None, Some(after)) => {
                        res += &format!(
                            "    * 😢 {} base build interrupted, to-check build {}",
                            self.pkg,
                            if *after { "passed" } else { "did not pass" },
                        )
                    }
                    (Some(before), None) => {
                        res += &format!(
                            "    * 😢 {} to-check build interrupted, base build {}",
                            self.pkg,
                            if *before { "passed" } else { "did not pass" },
                        )
                    }
                    (Some(true), Some(true)) => {
                        res += &format!("    * ✔ {} continued running successfully\n", test)
                    }
                    (Some(true), Some(false)) => {
                        res += &format!("    * ❌ {} started failing\n", test)
                    }
                    (Some(false), Some(true)) => {
                        res += &format!("    * 💚 {} started running successfully again\n", test)
                    }
                    (Some(false), Some(false)) => {
                        res += &format!("    * 😢 {} still fails\n", test)
                    }
                }
            }
            res += "\n";
        }
        res
    }
}

/// Returns true iff the build was successful
fn build(
    killer: &Receiver<()>,
    version: &str,
    pkg: &str,
) -> anyhow::Result<HashMap<String, Option<bool>>> {
    let test_names = crate::nix(
        killer,
        &[
            "eval",
            "--json",
            &format!(
                "(builtins.attrNames {}.{}.passthru.tests or {{}})",
                crate::nixpkgs(),
                pkg
            ),
        ],
    )
    .with_context(|| format!("recovering the list of tests for {}", pkg))?
    .as_ref()
    .and_then(|names| names.as_array())
    .and_then(|names| {
        names
            .iter()
            .map(|t| t.as_str().map(|t| t.to_string()))
            .collect::<Option<Vec<String>>>()
    })
    .unwrap_or_else(Vec::new);

    let mut res = HashMap::new();
    for test in test_names {
        println!("running test {}", test);
        let test_res = crate::run_nix(
            killer,
            false,
            &[
                "build",
                &crate::nix_eval_for(&format!("{}.passthru.tests.{}", pkg, test)),
            ],
        )
        .with_context(|| {
            format!(
                "building {} version of test {} in package {}",
                version, test, pkg
            )
        })?
        .map(|out| out.status.success());
        res.insert(test, test_res);
    }

    Ok(res)
}
