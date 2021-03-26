use nixpkgs_check::{checks, Check};

fn main() {
    let mut checks = vec![];
    let mut new_checks = vec![Box::new(checks::self_version::Chk::new()) as Box<dyn Check>];

    while !new_checks.is_empty() {
        // TODO: checkout “before”
        for c in new_checks.iter_mut() {
            c.run_before();
        }
        // TODO: checkout “after”
        for c in new_checks.iter_mut() {
            c.run_after();
        }
        let new_new_checks = new_checks
            .iter()
            .flat_map(|c| c.additional_needed_tests().into_iter())
            .collect::<Vec<_>>();
        checks.extend(new_checks.drain(..));
        new_checks = new_new_checks
            .into_iter()
            .filter(|nnc| !checks.iter().any(|c| nnc.uuid() == c.uuid()))
            .collect();
    }

    println!();
    println!();
    println!("Report to be pasted in the PR message");
    println!("-------------------------------------");
    println!();
    for c in checks {
        println!("{}", c.report());
    }
}
