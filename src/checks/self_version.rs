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

    fn run_before(&mut self) {}

    fn run_after(&mut self) {}

    fn additional_needed_tests(&self) -> Vec<Box<dyn crate::Check>> {
        vec![]
    }

    fn report(&self) -> String {
        format!(
            "**version:** `{} v{}`",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )
    }
}
