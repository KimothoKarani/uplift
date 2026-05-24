use chrono::NaiveDate;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const GA4_BASE: &str = "https://analyticsdata.googleapis.com/v1beta/properties";

#[derive(Debug, Clone, Copy)]
pub enum Ga4Metric {
    Sessions,
    ActiveUsers,
    Conversions,
    ScreenPageViews,
}

impl Ga4Metric {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sessions => "sessions",
            Self::ActiveUsers => "activeUsers",
            Self::Conversions => "conversions",
            Self::ScreenPageViews => "screenPageViews",
        }
    }
}

// ---- request types ----------------------------------
#[derive(Serialize)]
struct RunReportRequest<'a> {
    #[serde(rename = "dateRanges")]
    date_ranges: [DateRange; 1],
    dimensions: [Dimension; 1],
    metrics: [Metric<'a>; 1],
}

#[derive(Serialize)]
struct DateRange {
    #[serde(rename = "startDate")]
    start_date: String,
    #[serde(rename = "endDate")]
    end_date: String,
}

#[derive(Serialize)]
struct Dimension {
    name: &'static str,
}

#[derive(Serialize)]
struct Metric<'a> {
    name: &'a str,
}

// ----- response types ---------------------------
#[derive(Deserialize)]
struct RunReportResponse {
    rows: Option<Vec<Row>>,
}

#[derive(Deserialize)]
struct Row {
    #[serde(rename = "dimensionValues")]
    dimension_values: Vec<DimensionValue>,
    #[serde(rename = "metricValues")]
    metric_values: Vec<MetricValue>,
}

#[derive(Deserialize)]
struct DimensionValue {
    value: String,
}

#[derive(Deserialize)]
struct MetricValue {
    value: String,
}

// --- client ----------------------------------------
pub struct Ga4Client {
    http: Client,
}

impl Ga4Client {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    /// Fetch daily values for one metric over a date range.
    /// Returns rows sorted ascending by date.
    pub async fn fetch_daily_metric(
        &self,
        access_token: &str,
        property_id: &str,
        metric: Ga4Metric,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<(NaiveDate, f64)>> {
        let url = format!("{}/{}:runReport", GA4_BASE, property_id);

        let body = RunReportRequest {
            date_ranges: [DateRange {
                start_date: start.format("%Y-%m-%d").to_string(),
                end_date: end.format("%Y-%m-%d").to_string(),
            }],
            dimensions: [Dimension { name: "date" }],
            metrics: [Metric {
                name: metric.as_str(),
            }],
        };

        let resp = self
            .http
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(Error::Api { status, message });
        }

        let data: RunReportResponse = resp.json().await.map_err(|e| Error::Parse(e.to_string()))?;

        let rows = data.rows.ok_or(Error::NoData)?;
        if rows.is_empty() {
            return Err(Error::NoData);
        }

        let mut points = rows
            .into_iter()
            .map(|row| {
                let date_str = row
                    .dimension_values
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Parse("missing date dimension".into()))?
                    .value;
                let date = NaiveDate::parse_from_str(&date_str, "%Y%m%d")
                    .map_err(|e| Error::Parse(format!("invalid date '{}': {}", date_str, e)))?;

                let value: f64 = row
                    .metric_values
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Parse("missing metric value".into()))?
                    .value
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| {
                        Error::Parse(format!("invalid metric value: {}", e))
                    })?;

                Ok((date, value))
            })
            .collect::<Result<Vec<_>>>()?;

        points.sort_by_key(|(date, _)| *date);
        Ok(points)
    }
}
