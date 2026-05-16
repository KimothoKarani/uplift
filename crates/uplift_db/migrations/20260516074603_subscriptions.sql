-- Add migration script here
CREATE TABLE subscriptions (
    id                      UUID                    PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id         UUID                    NOT NULL REFERENCES organizations(id) ON DELETE CASCADE UNIQUE,
    stripe_subscription_id  UUID                    NOT NULL UNIQUE,
    tier                    TEXT                    NOT NULL,
    status                  TEXT                    NOT NULL,
    current_period_end      TIMESTAMPTZ             NOT NULL,
    updated_at              TIMESTAMPTZ             NOT NULL DEFAULT NOW()
);