use chrono::{Duration, NaiveDate, NaiveTime};
use uplift_core::timeseries::{DataPoint, TimeSeries};

use crate::error::{Error, Result};

/// Convert raw API output into a Timeseries, filling any missing days with zero.
pub fn into_timeseries(
    raw: Vec<(NaiveDate, f64)>,
    metric: impl Into<String>,
) -> Result<TimeSeries> {
    if raw.is_empty() {
        return Err(Error::NoData);
    }

    let filled = fill_gaps(raw);

    let points = filled
        .into_iter()
        .map(|(date, value)| DataPoint{
            timestamp: date.and_time(NaiveTime::MIN).and_utc(),
            value,
        })
        .collect();

    Ok(TimeSeries::new(metric, points))
}

fn fill_gaps(mut points: Vec<(NaiveDate, f64)>) -> Vec<(NaiveDate, f64)> {
    if points.is_empty() {
        return points;
    }

    points.sort_by_key(|(d, _)| *d);
    points.dedup_by_key(|(d, _)| *d);

    let start = points[0].0;
    let end = points[points.len() - 1].0;

    let mut result = Vec::new();
    let mut idx = 0;
    let mut current = start;

    while current <= end {
        if idx < points.len() && points[idx].0 == current {
            result.push(points[idx]);
            idx += 1;
        } else {
            result.push((current, 0.0));
        }
        current += Duration::days(1);
    }

    result

}