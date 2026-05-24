use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ModelVersion {
    ItsV1,
    BstsV2,
}

impl ModelVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ItsV1 => "its_v1",
            Self::BstsV2 => "bsts_v2",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointwiseEffect {
    pub timestamp: DateTime<Utc>,
    pub actual: f64,
    pub counterfactual: f64,
    pub effect: f64,
    pub effect_lower: f64,
    pub effect_upper: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub cumulative_effect: f64,
    pub cumulative_effect_lower: f64,
    pub cumulative_effect_upper: f64,
    pub relative_effect: f64,
    pub relative_effect_lower: f64,
    pub relative_effect_upper: f64,
    pub probability_of_effect: f64,
}

impl Summary {
    pub fn confidence_level(&self) -> ConfidenceLevel {
        if self.probability_of_effect >= 0.95 {
            ConfidenceLevel::High
        } else if self.probability_of_effect >= 0.80 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub model_version: ModelVersion,
    pub summary: Summary,
    pub pointwise: Vec<PointwiseEffect>,
    pub narrative: String,
}
