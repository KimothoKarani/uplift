use chrono::NaiveDate;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
const SC_BASE: &str = "https://searchconsole.googleapis.com/webmasters/v3/sites";

#[derive(Debug, Clone, Copy)]
pub enum ScMetric {
    Clicks,
    Impressions,
    Position,
}

// --- request types ---------------------------------------
#[derive(Serialize)]
struct QueryRequest {
    #[serde(rename = "startDate")]
    start_date: String,
    #[serde(rename = "endDate")]
    end_date: String,
    dimensions: [&'static str; 1],
    #[serde(rename = "rowLimit")]
    row_limit: u32,
}

// --- response types ---------------------
#[derive(Deserialize)]
struct QueryResponse {
    rows: Option<Vec<SearchRow>>,
}

#[derive(Deserialize)]
struct SearchRow {
    keys: Vec<String>,
    clicks: f64,
    impressions: f64,
    position: f64,
}

// ---- client ------------------------
pub struct SearchConsoleClient {
    http: Client,
}

impl SearchConsoleClient {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    /// Fetch daily values for one metric over a date range
    /// 'site_url' is the exact URL registered in Search Console,
    /// e.g. '"https://example.com"' or '"sc-domain:example.com"',
    pub async fn fetch_daily_metric(
        &self,
        access_token: &str,
        site_url: &str,
        metric: ScMetric,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<(NaiveDate, f64)>> {
        let encoded = site_url.replace(':', "%3A").replace('/', "%2F");
        let url = format!("{}/{}/searchAnalytics/query", SC_BASE, encoded);

        let body = QueryRequest {
            start_date: start.format("%Y-%m-%d").to_string(),
            end_date: end.format("%Y-%m-%d").to_string(),
            dimensions: ["date"],
            row_limit: 25_000,
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

        let data: QueryResponse = resp.json().await.map_err(|e| Error::Parse(e.to_string()))?;

        let rows = data.rows.ok_or(Error::NoData)?;
        if rows.is_empty() {
            return Err(Error::NoData);
        }

        let mut points = rows
            .into_iter()
            .map(|row| {
                let date_str = row
                    .keys
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Parse("missing date key".into()))?;
                let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|e| Error::Parse(format!("invalid date '{}': {}", date_str, e)))?;

                let value = match metric {
                    ScMetric::Clicks => row.clicks,
                    ScMetric::Impressions => row.impressions,
                    ScMetric::Position => row.position,
                };

                Ok((date, value))
            })
            .collect::<Result<Vec<_>>>()?;

        points.sort_by_key(|(date, _)| *date);
        Ok(points)
    }
}
