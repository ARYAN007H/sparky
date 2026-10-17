use super::*;

/// A stand-in flow row for handler tests.
pub(crate) fn stand_in_for_test(command: &str) -> CronJob {
    stand_in(command)
}
