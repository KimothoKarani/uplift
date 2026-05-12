use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub points: Vec<DataPoint>,
    pub metric: String,
}

impl TimeSeries {
    pub fn new(metric: impl Into<String>, points: Vec<DataPoint>) -> Self {
        Self {
            metric: metric.into(),
            points,
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        self.points.iter().map(|p| p.value)
    }

    pub fn timestamps(&self) -> impl Iterator<Item = &DateTime<Utc>> + '_ {
        self.points.iter().map(|p| &p.timestamp)
    }
}
