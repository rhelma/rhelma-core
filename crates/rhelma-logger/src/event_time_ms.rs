//! RFC3339 timestamps with millisecond precision (UTC "Z").
//!
//! Global log pipelines (ELK/Loki/Datadog/OpenSearch) generally index RFC3339 timestamps
//! and treat millisecond precision as the practical standard for logs (balance of size + utility).
//!
//! This module keeps *deserialization* flexible (accepts RFC3339), while enforcing
//! millisecond precision for *serialization*.

use serde::{Deserialize, Deserializer, Serializer};
use time::format_description::{self, well_known::Rfc3339};
use time::{OffsetDateTime, UtcOffset};

fn ms_format() -> &'static [time::format_description::FormatItem<'static>] {
    // Example: 2025-12-18T12:34:56.789Z
    static FMT: std::sync::OnceLock<Vec<time::format_description::FormatItem<'static>>> =
        std::sync::OnceLock::new();
    FMT.get_or_init(|| {
        format_description::parse(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
        )
        .expect("valid time format")
    })
}

pub fn serialize<S>(ts: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let utc = ts.to_offset(UtcOffset::UTC);
    let s = utc.format(ms_format()).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    // Accept any valid RFC3339 input (with or without fractional seconds).
    OffsetDateTime::parse(&s, &Rfc3339).map_err(serde::de::Error::custom)
}
