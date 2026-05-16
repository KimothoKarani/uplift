-- Add migration script here
CREATE TABLE organizations (
    id                  UUID                PRIMARY KEY DEFAULT gen_random_uuid(),
    name                TEXT                NOT NULL,
    slug                TEXT                NOT NULL UNIQUE,
    stripe_customer_id  TEXT                UNIQUE,
    created_at          TIMESTAMPTZ         NOT NULL DEFAULT NOW()
);

CREATE TABLE users (
    id                  UUID                PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID                NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email               TEXT                NOT NULL UNIQUE,
    display_name        TEXT,
    google_id           TEXT                UNIQUE,
    role                TEXT                NOT NULL DEFAULT 'member',
    created_at          TIMESTAMPTZ         NOT NULL DEFAULT NOW()
);