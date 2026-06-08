use serde::{Serializer, Deserializer, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Serialize SystemTime → RFC3339 string
pub fn serialize<S>(ts: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // تبدیل SystemTime → Duration since epoch (safe)
    let dur = ts
        .duration_since(UNIX_EPOCH)
        .map_err(serde::ser::Error::custom)?;

    // ساخت OffsetDateTime دقیقاً با نانوثانیه
    let nanos = dur.as_secs() as i128 * 1_000_000_000i128
        + dur.subsec_nanos() as i128;

    let odt = OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .map_err(serde::ser::Error::custom)?;

    let s = odt.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&s)
}

/// Deserialize RFC3339 string → SystemTime
pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    let odt = OffsetDateTime::parse(&s, &Rfc3339)
        .map_err(serde::de::Error::custom)?;

    // نانوثانیه از epoch
    let nanos = odt.unix_timestamp_nanos();

    if nanos < 0 {
        return Err(serde::de::Error::custom(
            "timestamp before UNIX_EPOCH is not supported",
        ));
    }

    let dur = Duration::from_nanos(nanos as u64);
    Ok(UNIX_EPOCH + dur)
}
