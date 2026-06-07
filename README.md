# Uplift

> Prove your marketing actually worked.

Uplift is a causal inference SaaS for SEO agencies. Instead of showing clients
a chart with a line going up, agencies can now say: "The content hub we launched
on March 1st caused a statistically significant 23% lift in organic sessions,
with 94% probability the effect is real and not noise."

Built with Rust end to end. Axum for the API, Leptos for server-side rendered
UI, SQLx for the database, and Apalis for background jobs. The causal model runs
inside the same binary with no Python dependency.

---

## The problem it solves

SEO agencies cannot prove causation. They can show traffic went up after a
campaign, but they cannot separate their work from seasonal trends, algorithm
updates, or pure coincidence. Clients know this, and it erodes trust and
justifies churn.

Uplift runs causal inference models against GA4 data to produce a statistically
rigorous answer to the question: did this specific intervention cause a
measurable lift?

Phase 1 ships Interrupted Time Series regression — valid causal inference,
explainable to non-statisticians, implementable now. Phase 2 upgrades to
Bayesian Structural Time Series (the method behind Google's published
CausalImpact paper by Brodersen et al., 2015), which handles non-linear trends,
time-varying seasonality, and structural uncertainty that ITS cannot.

---

## The causal model

### Phase 1: Interrupted Time Series (current)

Fits OLS on the pre-intervention period:

```
y_t = α + β₁t + β₂sin(2πt/7) + β₃cos(2πt/7) + β₄sin(4πt/7) + β₅cos(4πt/7) + γX_t + ε_t
```

Where:
- `α` — intercept
- `β₁t` — linear trend
- `β₂...β₅` — Fourier terms capturing weekly seasonality (web traffic has a strong 7-day cycle)
- `γX_t` — optional control variables (other metrics unaffected by the intervention)
- `ε_t` — residuals

**Counterfactual:** Project the fitted model forward into the post-period. This
is what would have happened without the intervention.

**Effect:** `actual_t - counterfactual_t` at each time point. Sum for the
cumulative effect.

**Confidence intervals:** Bootstrap the pre-period residuals. Resample with
replacement and add to the counterfactual predictions. Repeat 10,000 times.
The 2.5th and 97.5th percentiles of the bootstrapped cumulative effects are
the 95% confidence interval.

**P(effect > 0):** Fraction of bootstrap samples where the cumulative effect
is positive.

---

### Phase 2: Bayesian Structural Time Series (planned)

This is what Google's published CausalImpact method implements
(Brodersen et al., 2015). It handles non-linear trends, time-varying
seasonality, and structural uncertainty that ITS cannot.

**State space formulation:**

Observation equation:
```
y_t = Z_t'α_t + ε_t,    ε_t ~ N(0, σ²_ε)
```

State equation:
```
α_{t+1} = T_t α_t + R_t η_t,    η_t ~ N(0, Q_t)
```

Components of the state vector α_t:

Local linear trend:
```
μ_{t+1} = μ_t + δ_t + u_t,    u_t ~ N(0, σ²_μ)
δ_{t+1} = δ_t + v_t,           v_t ~ N(0, σ²_δ)
```

Seasonal component (weekly, S=7):
```
γ_{t+1} = -Σ_{s=1}^{6} γ_{t+1-s} + w_t,    w_t ~ N(0, σ²_γ)
```

Regression:
```
β'x_t    (spike-and-slab prior for automatic variable selection)
```

**Estimation: Gibbs sampler:**

1. Initialize all parameters
2. Repeat until convergence:
   - Run Kalman smoother → sample state trajectory given current parameters
   - Sample variance parameters (σ²_ε, σ²_μ, σ²_δ, σ²_γ) given state trajectory (conjugate Inverse-Gamma posteriors)
   - Sample regression coefficients β given state trajectory (spike-and-slab)
3. Run multiple chains in parallel via Rayon
4. Check convergence via R-hat statistic

**Counterfactual generation:**

After fitting on the pre-period [1, T₀]:
1. Initialize Kalman filter at the smoothed state at T₀
2. For each MCMC sample, propagate the state forward through the post-period [T₀+1, T]
3. Each sample gives one trajectory — the ensemble is the posterior predictive distribution over the counterfactual

**Causal effect output:**
- Point-wise effect: `α_t = y_t - E[ŷ_t | data]`
- Cumulative effect: `Σ α_t` with credible interval
- Relative effect: `(Σ α_t) / (Σ E[ŷ_t])`
- P(effect > 0): fraction of posterior samples with positive cumulative effect

---

## Tech stack

| Layer | Technology |
|---|---|
| Language | Rust (2024 edition) |
| Web framework | Axum 0.8 |
| Frontend | Leptos 0.7 (SSR + hydration) |
| Database | PostgreSQL 16 via SQLx 0.8 |
| Background jobs | Apalis 0.7 + apalis-sql (Postgres backend) |
| Auth | Google OAuth 2.0 |
| Payments | Stripe (Phase 8) |
| Deployment | Fly.io |

---

## Project structure

```
uplift/
├── crates/
│   ├── uplift_core/            # Causal inference engine — pure math, no I/O
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs        # Error enum + Result alias
│   │   │   ├── timeseries/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── series.rs   # TimeSeries<T> type, DataPoint
│   │   │   │   ├── decompose.rs # Seasonal decomposition (STL)
│   │   │   │   └── transform.rs # Log, difference, normalize
│   │   │   ├── model/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── its.rs      # Phase 1: Interrupted Time Series
│   │   │   │   ├── kalman.rs   # Phase 2: Kalman filter + smoother
│   │   │   │   ├── mcmc.rs     # Phase 2: Gibbs sampler
│   │   │   │   ├── bsts.rs     # Phase 2: BSTS model assembly
│   │   │   │   └── components.rs # Local level, seasonal, regression
│   │   │   ├── impact/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── analysis.rs # CausalImpact orchestration
│   │   │   │   └── report.rs   # ImpactReport, PointwiseEffect, Summary
│   │   │   └── narrative.rs    # Plain-English summary generation
│   ├── uplift_connectors/      # GA4 and Google OAuth API clients
│   ├── uplift_db/              # SQLx repositories and database migrations
│   ├── uplift_jobs/            # Apalis background workers
│   ├── uplift_api/             # Axum HTTP server — entry point binary
│   └── uplift_web/             # Leptos SSR frontend
├── docker-compose.yml          # Local Postgres
├── Leptos.toml                 # cargo-leptos config
└── Cargo.toml                  # Workspace root
```

---

## Prerequisites

- Rust (latest stable). Install via [rustup.rs](https://rustup.rs)
- Docker: for local Postgres
- sqlx-cli: for running migrations

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

---

## Getting started

### 1. Clone the repo

```bash
git clone https://github.com/KimothoKarani/uplift.git
cd uplift
```

### 2. Start Postgres

```bash
docker compose up -d
```

### 3. Set up environment variables

```bash
cp .env.example .env
```

Open `.env` and fill in:

| Variable | How to get it |
|---|---|
| `DATABASE_URL` | Already set for local Docker |
| `ENCRYPTION_KEY` | Run `openssl rand -base64 32` |
| `GOOGLE_CLIENT_ID` | Google Cloud Console → APIs & Services → Credentials |
| `GOOGLE_CLIENT_SECRET` | Same as above |
| `GOOGLE_REDIRECT_URI` | Set to `http://localhost:3000/auth/callback` |
| `APP_BASE_URL` | Set to `http://localhost:3000` |

Stripe and SMTP are optional for local development (leave them blank).

### 4. Run migrations

```bash
cargo sqlx migrate run --source crates/uplift_db/migrations
```

### 5. Run the app

```bash
cargo run -p uplift_api
```

Open [http://localhost:3000](http://localhost:3000).

---

## Google OAuth setup

1. Go to [console.cloud.google.com](https://console.cloud.google.com)
2. Create a project
3. Go to APIs & Services → OAuth consent screen → configure as External
4. Add your email as a test user under Audience
5. Go to Credentials → Create OAuth 2.0 Client ID → Web application
6. Set authorised redirect URI to `http://localhost:3000/auth/callback`
7. Copy the client ID and secret into `.env`

---

## Environment variables

```dotenv
DATABASE_URL=postgres://uplift:uplift@localhost:5432/uplift
ENCRYPTION_KEY=                    # openssl rand -base64 32
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
GOOGLE_REDIRECT_URI=http://localhost:3000/auth/callback
APP_BASE_URL=http://localhost:3000
RUST_LOG=uplift_api=debug,uplift_jobs=info,sqlx=warn

# Optional — leave blank to disable
STRIPE_SECRET_KEY=
STRIPE_WEBHOOK_SECRET=
SMTP_HOST=
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_FROM=
```

---

## How it works

```
User connects Google account via OAuth
    ↓
Uplift lists their GA4 properties (one per client website)
    ↓
User selects a property, metric, and intervention date
    ↓
API creates an analysis row (status: pending) and returns immediately
    ↓
Background job fetches daily GA4 data and caches it locally
    ↓
Causal model runs (ITS Phase 1 / BSTS Phase 2)
    ↓
Results saved: lift %, confidence interval, probability of effect, narrative
    ↓
User views the results page with chart and plain-English explanation
```

---

## Build phases

| Phase | Description | Status |
|---|---|---|
| 1 | Workspace architecture + ITS causal model | Done |
| 2 | Google OAuth + encrypted token storage | Done |
| 3 | Database layer (SQLx repositories) | Done |
| 4 | Background jobs (Apalis workers) | Done |
| 5 | GA4 connector (fetch daily metrics) | Done |
| 6 | Leptos SSR frontend | Done |
| 7 | BSTS causal model (Kalman filter + Gibbs sampler) | In progress |
| 8 | Stripe billing | Planned |
| 9 | Marketing site | Planned |

---

## Contributing

This is a private project. Access is by invitation only.
Contact [kimothokarani@gmail.com](mailto:kimothokarani@gmail.com) to request access.

---

## License

Private and confidential. All rights reserved.