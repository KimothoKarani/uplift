-- Add migration script here
CREATE TABLE time_series_data (
    id                  BIGSERIAL                   PRIMARY KEY,
    property_id         UUID                        NOT NULL REFERENCES ga4_properties(id) ON DELETE CASCADE,
    metric              TEXT                        NOT NULL,
    date                DATE                        NOT NULL,
    value               DOUBLE PRECISION            NOT NULL,
    fetched_at          TIMESTAMPTZ                 NOT NULL DEFAULT NOW(),
    UNIQUE (property_id, metric, date)
);

CREATE INDEX idx_timeseries_lookup ON time_series_data (property_id, metric, date);
