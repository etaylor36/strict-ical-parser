//! Validation of the RFC 5545 §3.3 DATE, DATE-TIME, and DURATION value
//! types. These are string formats, not `chrono`-style calendar objects —
//! all this module does is confirm a value is well-formed enough that a
//! consumer parsing it later won't hit a surprise.

/// Validate a DATE value: `YYYYMMDD` per RFC 5545 §3.3.4.
pub fn validate_date(s: &str) -> Result<(), String> {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{s:?} is not a valid DATE value, expected YYYYMMDD"));
    }
    let year: u32 = s[0..4].parse().unwrap();
    let month: u32 = s[4..6].parse().unwrap();
    let day: u32 = s[6..8].parse().unwrap();
    validate_ymd(year, month, day).map_err(|e| format!("{s:?} is not a valid DATE value: {e}"))
}

/// Validate a DATE-TIME value: `YYYYMMDDTHHMMSS` with an optional trailing
/// `Z` for UTC, per RFC 5545 §3.3.5.
pub fn validate_date_time(s: &str) -> Result<(), String> {
    let Some(t_idx) = s.find('T') else {
        return Err(format!("{s:?} is not a valid DATE-TIME value, missing 'T' time separator"));
    };
    let date_part = &s[..t_idx];
    let rest = &s[t_idx + 1..];
    if date_part.len() != 8 || !date_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{s:?} is not a valid DATE-TIME value, date part must be YYYYMMDD"));
    }
    let time_part = rest.strip_suffix('Z').unwrap_or(rest);
    if time_part.len() != 6 || !time_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{s:?} is not a valid DATE-TIME value, time part must be HHMMSS"));
    }

    let year: u32 = date_part[0..4].parse().unwrap();
    let month: u32 = date_part[4..6].parse().unwrap();
    let day: u32 = date_part[6..8].parse().unwrap();
    validate_ymd(year, month, day).map_err(|e| format!("{s:?} is not a valid DATE-TIME value: {e}"))?;

    let hour: u32 = time_part[0..2].parse().unwrap();
    let minute: u32 = time_part[2..4].parse().unwrap();
    // Seconds go up to 60 to allow for a leap second.
    let second: u32 = time_part[4..6].parse().unwrap();
    if hour > 23 {
        return Err(format!("{s:?} is not a valid DATE-TIME value: hour {hour} is out of range 00-23"));
    }
    if minute > 59 {
        return Err(format!("{s:?} is not a valid DATE-TIME value: minute {minute} is out of range 00-59"));
    }
    if second > 60 {
        return Err(format!("{s:?} is not a valid DATE-TIME value: second {second} is out of range 00-60"));
    }
    Ok(())
}

fn validate_ymd(year: u32, month: u32, day: u32) -> Result<(), String> {
    if !(1..=12).contains(&month) {
        return Err(format!("month {month} is out of range 01-12"));
    }
    let max_day = days_in_month(year, month);
    if day < 1 || day > max_day {
        return Err(format!("day {day} is out of range for {year:04}-{month:02}"));
    }
    Ok(())
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 0,
    }
}

/// Validate a DURATION value per RFC 5545 §3.3.6:
/// `["+" / "-"] "P" (dur-date / dur-time / dur-week)`.
pub fn validate_duration(s: &str) -> Result<(), String> {
    let unsigned = s.strip_prefix('+').or_else(|| s.strip_prefix('-')).unwrap_or(s);
    let Some(mut rest) = unsigned.strip_prefix('P') else {
        return Err(format!("{s:?} is not a valid DURATION value, missing 'P' designator"));
    };

    if let Some(w_idx) = rest.find('W') {
        let (digits, tail) = rest.split_at(w_idx);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || tail != "W" {
            return Err(format!("{s:?} is not a valid DURATION value"));
        }
        return Ok(());
    }

    let mut had_component = false;

    if let Some(d_idx) = rest.find('D') {
        let (digits, tail) = rest.split_at(d_idx);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("{s:?} is not a valid DURATION value"));
        }
        rest = &tail[1..];
        had_component = true;
    }

    if let Some(after_t) = rest.strip_prefix('T') {
        rest = after_t;
        let mut had_time_component = false;

        if let Some(h_idx) = rest.find('H') {
            let (digits, tail) = rest.split_at(h_idx);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("{s:?} is not a valid DURATION value"));
            }
            rest = &tail[1..];
            had_time_component = true;
        }
        if let Some(m_idx) = rest.find('M') {
            let (digits, tail) = rest.split_at(m_idx);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("{s:?} is not a valid DURATION value"));
            }
            rest = &tail[1..];
            had_time_component = true;
        }
        if let Some(s_idx) = rest.find('S') {
            let (digits, tail) = rest.split_at(s_idx);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!("{s:?} is not a valid DURATION value"));
            }
            rest = &tail[1..];
            had_time_component = true;
        }
        if !had_time_component {
            return Err(format!("{s:?} is not a valid DURATION value, 'T' with no H/M/S component"));
        }
        had_component = true;
    }

    if !had_component || !rest.is_empty() {
        return Err(format!("{s:?} is not a valid DURATION value"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_date_accepts_valid_calendar_date() {
        assert!(validate_date("20260315").is_ok());
    }

    #[test]
    fn validate_date_accepts_leap_day() {
        assert!(validate_date("20240229").is_ok());
    }

    #[test]
    fn validate_date_rejects_leap_day_in_non_leap_year() {
        assert!(validate_date("20230229").is_err());
    }

    #[test]
    fn validate_date_rejects_wrong_length() {
        assert!(validate_date("2026315").is_err());
    }

    #[test]
    fn validate_date_rejects_month_out_of_range() {
        assert!(validate_date("20261301").is_err());
    }

    #[test]
    fn validate_date_time_accepts_utc_form() {
        assert!(validate_date_time("20260315T090000Z").is_ok());
    }

    #[test]
    fn validate_date_time_accepts_local_form() {
        assert!(validate_date_time("20260315T090000").is_ok());
    }

    #[test]
    fn validate_date_time_accepts_leap_second() {
        assert!(validate_date_time("20260630T235960Z").is_ok());
    }

    #[test]
    fn validate_date_time_rejects_missing_t() {
        assert!(validate_date_time("20260315090000Z").is_err());
    }

    #[test]
    fn validate_date_time_rejects_hour_out_of_range() {
        assert!(validate_date_time("20260315T240000Z").is_err());
    }

    #[test]
    fn validate_duration_accepts_week() {
        assert!(validate_duration("P7W").is_ok());
    }

    #[test]
    fn validate_duration_accepts_day_and_time() {
        assert!(validate_duration("P15DT5H0M20S").is_ok());
    }

    #[test]
    fn validate_duration_accepts_negative_time_only() {
        assert!(validate_duration("-PT15M").is_ok());
    }

    #[test]
    fn validate_duration_rejects_missing_p() {
        assert!(validate_duration("15DT5H").is_err());
    }

    #[test]
    fn validate_duration_rejects_week_mixed_with_time() {
        assert!(validate_duration("P7WT5H").is_err());
    }

    #[test]
    fn validate_duration_rejects_empty_after_p() {
        assert!(validate_duration("P").is_err());
    }

    #[test]
    fn validate_duration_rejects_dangling_t() {
        assert!(validate_duration("PT").is_err());
    }

    #[test]
    fn validate_duration_rejects_trailing_garbage() {
        assert!(validate_duration("P1DT1HX").is_err());
    }
}
