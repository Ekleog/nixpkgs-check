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

    fn from_uuid_param(uuid: uuid::Uuid, param: &str) -> CheckId {
        CheckId(
            uuid.to_hyphenated()
                .encode_lower(&mut uuid::Uuid::encode_buffer())
                .to_string()
                + "-"
                + param,
        )
    }
}

pub trait Check {
    /// A UUID for this check (including any dynamic parameters it
    /// could have that might make it different from other checks of
    /// the same type)
    fn uuid(&self) -> CheckId;

    /// The human-meaningful name for this check
    fn name(&self) -> String;

    /// This is run on the checkout before the changes
    fn run_before(&mut self) -> anyhow::Result<()>;

    /// This is run on the checkout after the changes
    fn run_after(&mut self) -> anyhow::Result<()>;

    /// Returns the tests that are additionally needed
    fn additional_needed_tests(&self) -> Vec<Box<dyn Check>>;

    /// Generate the report
    fn report(&self) -> String;
}
