use sqlx::postgres::types::PgInterval;

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
