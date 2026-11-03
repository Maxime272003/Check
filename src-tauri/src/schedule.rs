use chrono::{Datelike, Duration, Local, LocalResult, NaiveDate, TimeZone};

use crate::model::Schedule;

fn parse_time(t: &str) -> (u32, u32) {
    let mut it = t.split(':');
    let h = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0).min(23);
    let m = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0).min(59);
    (h, m)
}

/// Horodatage local (ms) d'une date à hh:mm, en tolérant les changements d'heure.
fn at(d: NaiveDate, hh: u32, mm: u32) -> Option<i64> {
    let ndt = d.and_hms_opt(hh, mm, 0)?;
    match Local.from_local_datetime(&ndt) {
        LocalResult::Single(t) => Some(t.timestamp_millis()),
        LocalResult::Ambiguous(a, _) => Some(a.timestamp_millis()),
        LocalResult::None => Local
            .from_local_datetime(&(ndt + Duration::hours(1)))
            .earliest()
            .map(|t| t.timestamp_millis()),
    }
}

/// Prochaine remise à zéro strictement après `after` (ms). `None` si jamais.
pub fn next_reset(s: &Schedule, after: i64) -> Option<i64> {
    let after_dt = Local.timestamp_millis_opt(after).single()?;
    let (hh, mm) = parse_time(&s.time);
    let start = after_dt.date_naive();
    match s.kind.as_str() {
        "interval" => Some(after + i64::from(s.hours.max(1)) * 3_600_000),
        "daily" => (0..3)
            .filter_map(|i| at(start + Duration::days(i), hh, mm))
            .find(|&c| c > after),
        "weekly" => (0..15)
            .map(|i| start + Duration::days(i))
            .filter(|d| d.weekday().num_days_from_monday() == s.day.min(6))
            .filter_map(|d| at(d, hh, mm))
            .find(|&c| c > after),
        "monthly" => {
            let dom = s.dom.clamp(1, 28);
            (0..3).find_map(|i| {
                let m0 = start.month0() + i;
                let y = start.year() + (m0 / 12) as i32;
                let d = NaiveDate::from_ymd_opt(y, m0 % 12 + 1, dom)?;
                at(d, hh, mm).filter(|&c| c > after)
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekly_is_within_a_week() {
        let now = Local::now().timestamp_millis();
        let s = Schedule { kind: "weekly".into(), day: 0, time: "00:00".into(), ..Default::default() };
        let n = next_reset(&s, now).unwrap();
        assert!(n > now && n - now <= 7 * 86_400_000 + 3_600_000);
        let d = Local.timestamp_millis_opt(n).unwrap();
        assert_eq!(d.weekday().num_days_from_monday(), 0);
    }

    #[test]
    fn daily_and_monthly_and_none() {
        let now = Local::now().timestamp_millis();
        let d = Schedule { kind: "daily".into(), time: "06:30".into(), ..Default::default() };
        assert!(next_reset(&d, now).unwrap() - now <= 86_400_000 + 3_600_000);
        let m = Schedule { kind: "monthly".into(), dom: 1, time: "09:00".into(), ..Default::default() };
        assert_eq!(Local.timestamp_millis_opt(next_reset(&m, now).unwrap()).unwrap().day(), 1);
        assert!(next_reset(&Schedule::default(), now).is_none());
        let i = Schedule { kind: "interval".into(), hours: 48, ..Default::default() };
        assert_eq!(next_reset(&i, now).unwrap() - now, 48 * 3_600_000);
    }
}
