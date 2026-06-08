pub(crate) type ProviderTestResult = (usize, usize, bool, Option<u16>, Option<u64>, String);
pub(super) type ProviderTestMessage = ProviderTestResult;
pub(super) type ProbeOutcome = (bool, Option<u16>, Option<u64>, String);
