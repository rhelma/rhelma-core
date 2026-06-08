use crate::config::PerformanceProfile;

/// Derived performance configuration based on high-level profile.
#[derive(Debug, Clone, Copy)]
pub struct LoggerPerformanceConfig {
    /// Field `sample_rate`.
    pub sample_rate: f64,
}

impl LoggerPerformanceConfig {
    pub fn from_profile(profile: PerformanceProfile, sampling_rate: f64) -> Self {
        let sample_rate = match profile {
            PerformanceProfile::LowLatency => sampling_rate.min(0.5),
            PerformanceProfile::Balanced => sampling_rate,
            PerformanceProfile::HighThroughput => sampling_rate.max(0.8),
        };
        Self { sample_rate }
    }
}
