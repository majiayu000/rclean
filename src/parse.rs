use std::time::Duration;

use crate::error::ParseError;

pub fn parse_size(raw: &str) -> Result<u64, ParseError> {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() {
        return Err(ParseError::InvalidSize("size cannot be empty".to_string()));
    }

    let split_at = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    let number = &value[..split_at];
    let unit = &value[split_at..];

    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return Err(ParseError::InvalidSize(format!(
            "invalid size '{raw}'. Use values like 0, 100mb, or 1g"
        )));
    }

    let amount: u64 = number.parse().map_err(|_| {
        ParseError::InvalidSize(format!("invalid size '{raw}'. Number is too large"))
    })?;

    let multiplier = match unit {
        "" | "b" => 1,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024_u64.pow(2),
        "g" | "gb" => 1024_u64.pow(3),
        "t" | "tb" => 1024_u64.pow(4),
        _ => {
            return Err(ParseError::InvalidSize(format!(
                "invalid size unit '{unit}'. Use b, kb, mb, gb, or tb"
            )));
        }
    };

    amount
        .checked_mul(multiplier)
        .ok_or_else(|| ParseError::InvalidSize(format!("invalid size '{raw}'. Value is too large")))
}

pub fn parse_duration(raw: &str) -> Result<Duration, ParseError> {
    let value = raw.trim().to_ascii_lowercase();
    if value.len() < 2 {
        return Err(ParseError::InvalidDuration(format!(
            "invalid duration '{raw}'. Use values like 30d, 6m, or 1y"
        )));
    }

    let (number, unit) = value.split_at(value.len() - 1);
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return Err(ParseError::InvalidDuration(format!(
            "invalid duration '{raw}'. Use values like 30d, 6m, or 1y"
        )));
    }

    let amount: u64 = number.parse().map_err(|_| {
        ParseError::InvalidDuration(format!("invalid duration '{raw}'. Number is too large"))
    })?;
    let seconds = match unit {
        "s" => 1,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        "w" => 7 * 24 * 60 * 60,
        "m" => 30 * 24 * 60 * 60,
        "y" => 365 * 24 * 60 * 60,
        _ => {
            return Err(ParseError::InvalidDuration(format!(
                "invalid duration unit '{unit}'. Use s, h, d, w, m, or y"
            )));
        }
    };

    amount
        .checked_mul(seconds)
        .map(Duration::from_secs)
        .ok_or_else(|| {
            ParseError::InvalidDuration(format!("invalid duration '{raw}'. Value is too large"))
        })
}

pub fn parse_timeout_duration(raw: &str) -> Result<Duration, ParseError> {
    let value = raw.trim().to_ascii_lowercase();
    if value.len() < 2 {
        return Err(ParseError::InvalidDuration(format!(
            "invalid timeout '{raw}'. Use values like 5s, 1m, or 1h"
        )));
    }

    let (number, unit) = value.split_at(value.len() - 1);
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return Err(ParseError::InvalidDuration(format!(
            "invalid timeout '{raw}'. Use values like 5s, 1m, or 1h"
        )));
    }

    let amount: u64 = number.parse().map_err(|_| {
        ParseError::InvalidDuration(format!("invalid timeout '{raw}'. Number is too large"))
    })?;
    let seconds = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 60 * 60,
        _ => {
            return Err(ParseError::InvalidDuration(format!(
                "invalid timeout unit '{unit}'. Use s, m, or h"
            )));
        }
    };

    amount
        .checked_mul(seconds)
        .map(Duration::from_secs)
        .ok_or_else(|| {
            ParseError::InvalidDuration(format!("invalid timeout '{raw}'. Value is too large"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_size_units() {
        assert_eq!(parse_size("0").unwrap(), 0);
        assert_eq!(parse_size("1kb").unwrap(), 1024);
        assert_eq!(parse_size("2m").unwrap(), 2 * 1024 * 1024);
        assert_eq!(parse_size("3GB").unwrap(), 3 * 1024 * 1024 * 1024);
    }

    #[test]
    fn rejects_bad_size() {
        assert!(parse_size("mb").is_err());
        assert!(parse_size("10xb").is_err());
    }

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration("60s").unwrap(), Duration::from_secs(60));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("2d").unwrap(), Duration::from_secs(172800));
        assert_eq!(parse_duration("1w").unwrap(), Duration::from_secs(604800));
    }

    #[test]
    fn rejects_bad_duration() {
        assert!(parse_duration("10").is_err());
        assert!(parse_duration("x1d").is_err());
    }

    #[test]
    fn parses_timeout_duration_units() {
        assert!(matches!(
            parse_timeout_duration("5s"),
            Ok(value) if value == Duration::from_secs(5)
        ));
        assert!(matches!(
            parse_timeout_duration("1m"),
            Ok(value) if value == Duration::from_secs(60)
        ));
        assert!(matches!(
            parse_timeout_duration("2h"),
            Ok(value) if value == Duration::from_secs(7200)
        ));
    }

    #[test]
    fn timeout_duration_minutes_are_not_scan_age_months() {
        let timeout = parse_timeout_duration("1m");
        let scan_age = parse_duration("1m");
        assert!(matches!(
            (timeout, scan_age),
            (Ok(timeout), Ok(scan_age)) if timeout != scan_age
        ));
    }

    #[test]
    fn rejects_bad_timeout_duration() {
        assert!(parse_timeout_duration("10").is_err());
        assert!(parse_timeout_duration("1d").is_err());
    }
}
