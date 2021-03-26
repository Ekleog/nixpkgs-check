pub mod checks;

#[derive(PartialEq, Eq)]
pub struct CheckId(String);

impl CheckId {
    fn from_uuid(uuid: uuid::Uuid) -> CheckId {
        CheckId(
            uuid.to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer())
                .to_string(),
        )
    }
}

pub trait Check {
    /// A UUID for this check (including any dynamic parameters it
    /// could have that might make it different from other checks of
    /// the same type)
    fn uuid(&self) -> CheckId;

    /// This is run on the checkout before the changes
    fn run_before(&mut self);

    /// This is run on the checkout after the changes
    fn run_after(&mut self);

    /// Returns the tests that are additionally needed
    fn additional_needed_tests(&self) -> Vec<Box<dyn Check>>;

    /// Generate the report
    fn report(&self) -> String;
}
