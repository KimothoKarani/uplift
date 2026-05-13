use crate::impact::report::{ConfidenceLevel, ImpactReport, ModelVersion};

pub fn generate(report: &ImpactReport, metric: &str) -> String {
    let s = &report.summary;
    let n_days = report.pointwise.len();
    let prob_pct = (s.probability_of_effect * 100.0).round() as u32;
    let pct = (s.relative_effect * 100.0).abs();
    let abs_effect = s.cumulative_effect.abs();

    let direction = if s.cumulative_effect >= 0.0 {
        "increased"
    } else {
        "decreased"
    };

    let confidence_word = match s.confidence_level() {
        ConfidenceLevel::High => "HIGH",
        ConfidenceLevel::Medium => "MEDIUM",
        ConfidenceLevel::Low => "LOW",
    };

    let model_name = match report.model_version {
        ModelVersion::ItsV1 => "interrupted time series regression",
        ModelVersion::BstsV2 => "Bayesian structural time series",
    };

    let fmt_bound = |v: f64| {
        if v >= 0.0 {
            format!("+{:.0}", v)
        } else {
            format!("{:.0}", v)
        }
    };

    let ci = format!(
        "{} to {}",
        fmt_bound(s.cumulative_effect_lower),
        fmt_bound(s.cumulative_effect_upper),
    );

    format!(
        "Over the {n_days} days following the intervention, {metric} {direction} by \
        {pct:.1} (approximately {abs_effect} units) relative to the modelled \
        counterfactual - the trajectory the metric would have followed had the \
        intervention not occurred. A {model_name} analysis assigns a {prob_pct}% \
        probability that this effect is genuine and not attributable to natural \
        variation, placing confidence at {confidence_word}. The 95% confidence\
        interval on the cumulative effect ranges from {ci}."
    )


}