-- Add migration script here
CREATE TABLE ga4_properties (
    id                      UUID                        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id         UUID                        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    google_connection_id    UUID                        NOT NULL REFERENCES google_connections(id) ON DELETE CASCADE,
    ga4_property_id         TEXT                        NOT NULL,
    display_name            TEXT                        NOT NULL,
    website_url             TEXT,
    created_at              TIMESTAMPTZ                 NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, ga4_property_id)
);