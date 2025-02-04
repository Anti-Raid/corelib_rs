use rand::{distributions::Alphanumeric, Rng};
use sqlx::postgres::types::PgInterval;
use std::collections::HashMap;
use std::sync::LazyLock;

pub fn create_special_allocation_from_str(
    special_allocations: &str,
) -> Result<HashMap<String, u32>, crate::Error> {
    let split = special_allocations.split(',').collect::<Vec<&str>>();

    if !split.is_empty() {
        let mut map = HashMap::new();

        for v in split {
            if v.is_empty() {
                continue;
            }

            let split = v.split('=').collect::<Vec<&str>>();

            if split.len() != 2 {
                return Err("Invalid special allocation format".into());
            }

            let channel_id = split[0].to_string();
            let number = split[1].parse::<u32>()?;

            map.insert(channel_id, number);
        }

        Ok(map)
    } else {
        Ok(HashMap::new())
    }
}

pub fn pg_interval_to_secs(i: PgInterval) -> i64 {
    i.microseconds / 1000000 + ((i.days * 86400) as i64) + ((i.months * 2628000) as i64)
}

pub fn pg_interval_to_chrono_duration(i: PgInterval) -> chrono::Duration {
    let secs = pg_interval_to_secs(i);

    chrono::Duration::from_std(std::time::Duration::from_secs(
        secs.try_into().unwrap_or_default(),
    ))
    .unwrap_or_default()
}

pub fn secs_to_pg_interval(secs: i64) -> PgInterval {
    PgInterval {
        microseconds: secs * 1000000,
        days: (secs / 86400) as i32,
        months: (secs / 2628000) as i32,
    }
}

pub fn chrono_duration_to_pg_interval(d: chrono::Duration) -> PgInterval {
    let secs = d.num_seconds();

    secs_to_pg_interval(secs)
}

pub fn secs_to_pg_interval_u64(secs: u64) -> PgInterval {
    // Check if the value is too large to fit in an i64
    if secs > i64::MAX as u64 {
        // If it is, return the maximum value
        return PgInterval {
            microseconds: i64::MAX,
            days: i32::MAX,
            months: i32::MAX,
        };
    }

    secs_to_pg_interval(secs as i64)
}

pub fn parse_pg_interval(i: PgInterval) -> String {
    let seconds = pg_interval_to_secs(i);

    let dur = std::time::Duration::from_secs(seconds.try_into().unwrap_or_default());

    format!("{:?}", dur)
}

#[derive(PartialEq, Debug)]
pub enum Unit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl Unit {
    /// Convert the unit to seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            Unit::Seconds => 1,
            Unit::Minutes => 60,
            Unit::Hours => 3600,
            Unit::Days => 86400,
            Unit::Weeks => 604800,
        }
    }

    /// Same as to_seconds but returns an i64 instead of a u64 for easier use with sqlx
    pub fn to_seconds_i64(&self) -> i64 {
        match self {
            Unit::Seconds => 1,
            Unit::Minutes => 60,
            Unit::Hours => 3600,
            Unit::Days => 86400,
            Unit::Weeks => 604800,
        }
    }
}

impl TryFrom<&str> for Unit {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "seconds" => Ok(Unit::Seconds),
            "second" => Ok(Unit::Seconds), // Allow "second" as a shorthand for "seconds"
            "secs" => Ok(Unit::Seconds),   // Allow "secs" as a shorthand for "seconds"
            "sec" => Ok(Unit::Seconds),    // Allow "sec" as a shorthand for "seconds"
            "s" => Ok(Unit::Seconds),      // Allow "s" as a shorthand for "seconds"
            "minutes" => Ok(Unit::Minutes),
            "minute" => Ok(Unit::Minutes), // Allow "minute" as a shorthand for "minutes"
            "mins" => Ok(Unit::Minutes),   // Allow "mins" as a shorthand for "minutes"
            "min" => Ok(Unit::Minutes),    // Allow "min" as a shorthand for "minutes"
            "m" => Ok(Unit::Minutes),      // Allow "m" as a shorthand for "minutes"
            "hours" => Ok(Unit::Hours),
            "hour" => Ok(Unit::Hours), // Allow "hour" as a shorthand for "hours"
            "hrs" => Ok(Unit::Hours),  // Allow "hrs" as a shorthand for "hours"
            "hr" => Ok(Unit::Hours),   // Allow "hr" as a shorthand for "hours"
            "h" => Ok(Unit::Hours),    // Allow "h" as a shorthand for "hours"
            "days" => Ok(Unit::Days),
            "day" => Ok(Unit::Days), // Allow "day" as a shorthand for "days"
            "d" => Ok(Unit::Days),   // Allow "d" as a shorthand for "days"
            "weeks" => Ok(Unit::Weeks),
            "week" => Ok(Unit::Weeks), // Allow "week" as a shorthand for "weeks"
            "w" => Ok(Unit::Weeks),    // Allow "w" as a shorthand for "weeks"
            _ => Err("Invalid unit".into()),
        }
    }
}

/// Given a string of the format <number> days/hours/minutes/seconds, parse it into a u64 of seconds
///
/// This function should handle both spaced and non-spaced formats
pub fn parse_duration_string(s: &str) -> Result<(u64, Unit), crate::Error> {
    let mut number: u64 = 0;
    let mut unit = String::new();

    // Keep looping adding up each number until we hit a non-number which gets added to unit
    for c in s.chars() {
        if c.is_numeric() {
            number = number * 10 + c.to_digit(10).ok_or("Cannot convert to integer")? as u64;
        } else {
            if c == ' ' {
                continue;
            }

            unit.push(c);
        }
    }

    let unit = Unit::try_from(unit.as_str())?;

    Ok((number, unit))
}

pub static REPLACE_CHANNEL: LazyLock<Vec<(&'static str, &'static str)>> =
    LazyLock::new(|| vec![("<#", ""), (">", "")]);

pub static REPLACE_USER: LazyLock<Vec<(&'static str, &'static str)>> =
    LazyLock::new(|| vec![("<@", ""), ("!", ""), (">", "")]);

pub static REPLACE_ROLE: LazyLock<Vec<(&'static str, &'static str)>> =
    LazyLock::new(|| vec![("<@", ""), ("&", ""), (">", "")]);

/// Parse a numeric list from a string without knowing its separator
pub fn parse_numeric_list<T: std::str::FromStr + Send + Sync>(
    s: &str,
    replace: &[(&'static str, &'static str)],
) -> Result<Vec<T>, T::Err> {
    let mut list = Vec::new();
    let mut number = String::new();

    for c in s.chars() {
        if c.is_numeric() {
            number.push(c);
        } else if !number.is_empty() {
            for (from, to) in replace {
                number = number.replace(from, to);
            }
            list.push(number.parse::<T>()?);
            number.clear();
        }
    }

    if !number.is_empty() {
        list.push(number.parse::<T>()?);
    }

    Ok(list)
}

/// Parse a numeric list from a string without knowing its separator, returning a string instead of a number
#[allow(dead_code)]
pub fn parse_numeric_list_to_str<T: std::fmt::Display + std::str::FromStr + Send + Sync>(
    s: &str,
    replace: &[(&'static str, &'static str)],
) -> Result<Vec<String>, T::Err> {
    parse_numeric_list::<T>(s, replace).map(|v| v.into_iter().map(|v| v.to_string()).collect())
}

/// Returns a random string of length ``length``
pub fn gen_random(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    s
}
