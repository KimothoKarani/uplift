use chrono::{DateTime, Utc};
use rand::{Rng, SeedableRng};
use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::model::ItsModel;
use crate::timeseries::{DataPoint, TimeSeries};
use super::report::{ImpactReport, ModelVersion, PointwiseEffect, Summary};

const BOOTSTRAP_SAMPLES: usize = 10_000;

pub fn run_analysis(
    series: &TimeSeries,
    intervention_date: DateTime<Utc>,
    alpha: f64,
) -> Result<ImpactReport> {
    let (pre_points, post_points): (Vec<_>, Vec<_>) = 
        series.points.iter().partition(|p| p.timestamp < intervention_date);

    if post_points.is_empty() {
        return Err(Error::InvalidInterventionDate("no data points after the intervention date".into(),
    ));
    }

    let pre_series = TimeSeries::new(
        series.metric.clone(),
        pre_points
            .iter()
            .map(|p| DataPoint{timestamp: p.timestamp, value: p.value})
            .collect(),
        );
    
    let actual_post: Vec<f64> = post_points.iter().map(|p| p.value).collect();
    let post_timestamps: Vec<DateTime<Utc>> = post_points.iter().map(|p| p.timestamp).collect();

    let model = ItsModel::fit(&pre_series)?;
    let counterfactual = model.predict(actual_post.len());
    let residuals = model.residuals().to_vec();
    let n_residuals = residuals.len();
    let n_post = actual_post.len();

    // Each bootstrap sample draws n_post residuals and computes cumulative
    // effect under that noise trajectory. Parallelised across 10k samples.
    let bootstrap_cumulative: Vec<f64> = (0..BOOTSTRAP_SAMPLES)
        .into_par_iter()
        .map(|seed| {
            let mut rng = rand::rngs::SmallRng::seed_from_u64(seed as u64);
            (0..n_post)
                .map(|t| {
                    let noise = residuals[rng.random_range(0..n_residuals)];
                    actual_post[t] - (counterfactual[t] + noise)
                })
                .sum::<f64>()
        })
        .collect();

    let cumulative_effect: f64 = actual_post
        .iter()
        .zip(&counterfactual)
        .map(|(a, c)| a - c)
        .sum();

    let mut sorted = bootstrap_cumulative;
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let lo = ((alpha / 2.0) * BOOTSTRAP_SAMPLES as f64) as usize;
    let hi = ((1.0 - alpha / 2.0) * BOOTSTRAP_SAMPLES as f64)
        .min(BOOTSTRAP_SAMPLES as f64 - 1.0) as usize;

    let cumulative_effect_lower = sorted[lo];
    let cumulative_effect_upper = sorted[hi];

    let probability_of_effect = 
        sorted.iter().filter(|&&e| e > 0.0).count() as f64 / BOOTSTRAP_SAMPLES as f64;
    
    let counterfactual_sum: f64 = counterfactual.iter().sum();
    let relative = |v: f64| {
        if counterfactual_sum != 0.0 {v / counterfactual_sum} else {
            0.0
        }
    };

    // Pointwise CI: residual quantiles used as a constant-width noise envelope.
    // effect_lower = effect - r_hi (counterfactual was higher -> effect was smaller)
    // effect_upper = effect - r_lo (counterfactual was lower -> effect was larger)
    let mut sorted_residuals = residuals;
    sorted_residuals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let r_lo = sorted_residuals[((alpha / 2.0) * n_residuals as f64) as usize];
    let r_hi = sorted_residuals[
        ((1.0 - alpha / 2.0) * n_residuals as f64).min(n_residuals as f64 - 1.0) as usize
    ];

    let pointwise: Vec<PointwiseEffect> = (0..n_post)
        .map(|t| {
            let effect = actual_post[t] - counterfactual[t];
            PointwiseEffect {
                timestamp: post_timestamps[t],
                actual: actual_post[t],
                counterfactual: counterfactual[t],
                effect,
                effect_lower: effect - r_hi,
                effect_upper: effect - r_lo,
            }
        })
        .collect();

    let report = ImpactReport { 
        model_version: ModelVersion::ItsV1, 
        summary: Summary { 
            cumulative_effect, 
            cumulative_effect_lower, 
            cumulative_effect_upper, 
            relative_effect: relative(cumulative_effect), 
            relative_effect_lower: relative(cumulative_effect_lower), 
            relative_effect_upper: relative(cumulative_effect_upper), 
            probability_of_effect
        },
        pointwise,
        narrative: String::new(),
    };

    Ok(ImpactReport { 
        narrative: crate::narrative::generate(&report, &series.metric),
        ..report
         })
}
