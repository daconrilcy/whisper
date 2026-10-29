#[cfg(test)]
fn speech_limit_samples(enabled: bool, minutes: u64) -> Option<u64> {
    enabled.then(|| minutes.max(1).saturating_mul(60 * 16_000))
}
