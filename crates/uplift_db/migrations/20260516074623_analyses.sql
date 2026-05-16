-- Add migration script here
CREATE TABLE analyses (
    id                      UUID                    PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id         UUID                    NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    property_id             UUID                    NOT NULL REFERENCES ga4_properties(id) ON DELETE CASCADE,
    metric                  TEXT                    NOT NULL,
    intervention_date       DATE                    NOT NULL,
    pre_period_start        DATE                    NOT NULL,
    pre_period_end          DATE                    NOT NULL,
    post_period_start       DATE                    NOT NULL,
    post_period_end         DATE                    NOT NULL,
    description             TEXT                    NOT NULL,
    status                  TEXT                    NOT NULL DEFAULT 'pending'
                                CHECK (status IN ('pending', 'running', 'complete', 'failed')),
    error_message           TEXT,
    created_by              UUID                    NOT NULL REFERENCES users(id),
    created_at              TIMESTAMPTZ             NOT NULL DEFAULT NOW() 
);

CREATE TABLE analysis_results (
    id                      UUID                    PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id             UUID                    NOT NULL REFERENCES analyses(id) ON DELETE CASCADE UNIQUE,
    model_version           TEXT                    NOT NULL,
    cumulative_effect       DOUBLE PRECISION        NOT NULL,
    cumulative_effect_lower DOUBLE PRECISION        NOT NULL,
    cumulative_effect_upper DOUBLE PRECISION        NOT NULL,
    relative_effect         DOUBLE PRECISION        NOT NULL,
    relative_effect_lower   DOUBLE PRECISION        NOT NULL,
    relative_effect_upper   DOUBLE PRECISION        NOT NULL,
    probability_of_effect   DOUBLE PRECISION        NOT NULL,
    pointwise_effects       JSONB                   NOT NULL,
    narrative               TEXT                    NOT NULL,
    computed_at             TIMESTAMPTZ             NOT NULL DEFAULT NOW()
);