-- Add migration script here
CREATE TABLE google_connections (
    id                      UUID                    PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id         UUID                    NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    google_account_email    TEXT                    NOT NULL,
    access_token            TEXT                    NOT NULL,
    refresh_token           TEXT                    NOT NULL,
    token_expires_at        TIMESTAMPTZ             NOT NULL,
    created_at              TIMESTAMPTZ             NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, google_account_email)
);