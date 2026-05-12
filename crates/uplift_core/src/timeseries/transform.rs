use crate::error::{Error, Result};
use super::series::TimeSeries;

/// Replace each value with its natural log. Useful when data spans
/// several orders of magnitude (e.g. pageviews). Fails if any value <= 0.
pub fn log_transform(series: &TimeSeries) -> Result<Vec<f64>> {
    series.values().map(|v| {
        if v <= 0.0 {
            Err(Error::NumericalError(format!("log of non-positive value: {v}")))
        } else {
            Ok(v.ln())
        }
    }).collect()
}

/// Subtract each value from the previous one. Removes a linear trend
/// so the series becomes stationary (required by most time-series models)
pub fn difference(series: &TimeSeries) -> Vec<f64> {
    let vals: Vec<f64> = series.values().collect();
    vals.windows(2).map(|w| w[1] - w[0]).collect()
}

///Scale all values to the [0,1] range using min-max normalization.
pub fn normalize(series: &TimeSeries) -> Result<Vec<f64>> {
    let vals: Vec<f64> = series.values().collect();
    let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    if range == 0.0 {
        return Err(Error::NumericalError("cannot normalize a constant series".into()));
    }

    Ok(vals.iter().map(|v| (v - min) / range).collect())
}

