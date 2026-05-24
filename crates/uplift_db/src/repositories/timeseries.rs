use chrono::{NaiveDate, NaiveTime};
use sqlx::PgPool;
use uuid::Uuid;

use uplift_core::timeseries::{DataPoint, TimeSeries};

use crate::error::Result;

pub struct TimeSeriesRepo;

impl TimeSeriesRepo {
    /// Bulk-upsert daily data points fetched from GA4
    /// Safe to call repeatedly - duplicate dates update in place.
    pub async fn upsert_many(
        pool: &PgPool,
        property_id: Uuid,
        metric: &str,
        points: &[DataPoint],
    ) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }

        let property_ids: Vec<Uuid> = points.iter().map(|_| property_id).collect();
        let metrics: Vec<&str> = points.iter().map(|_| metric).collect();
        let dates: Vec<NaiveDate> = points.iter().map(|p| p.timestamp.date_naive()).collect();
        let values: Vec<f64> = points.iter().map(|p| p.value).collect();

        sqlx::query(
            r#"
            INSERT INTO time_series_data (property_id, metric, date, value)
            SELECT * FROM UNNEST($1::uuid[], $2::text[], $3::date[], $4::float8[])
            ON CONFLICT (property_id, metric, date) DO UPDATE SET
                value   = EXCLUDED.value,
                fetched_at = NOW()
            "#,
        )
        .bind(&property_ids)
        .bind(&metrics)
        .bind(&dates)
        .bind(values)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Read a date range back out as a Timeseries - ready to pass directly to run_analysis().
    pub async fn get_range(
        pool: &PgPool,
        property_id: Uuid,
        metric: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<TimeSeries> {
        struct Row {
            date: NaiveDate,
            value: f64,
        }

        let rows = sqlx::query_as!(
            Row,
            r#"
            SELECT date, value
            FROM time_series_data
            WHERE property_id = $1
                AND metric   = $2
                AND date BETWEEN $3 AND $4
            ORDER BY date
            "#,
            property_id,
            metric,
            start,
            end,
        )
        .fetch_all(pool)
        .await?;

        let points = rows
            .into_iter()
            .map(|r| DataPoint {
                timestamp: r.date.and_time(NaiveTime::MIN).and_utc(),
                value: r.value,
            })
            .collect();

        Ok(TimeSeries::new(metric, points))
    }
}
