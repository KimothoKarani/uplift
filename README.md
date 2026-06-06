# Uplift

> Prove your marketing actually worked.

Uplift is a causal inference SaaS for SEO agencies. Instead of showing clients
a chart with a line going up, agencies can now say: "The content hub we launched
on March 1st caused a statistically significant 23% lift in organic sessions,
with 94% probability the effect is real and not noise."

Built with Rust end to end — Axum for the API, Leptos for server-side rendered
UI, SQLx for the database, and Apalis for background jobs. The causal model runs
inside the same binary with no Python dependency.

---

## The problem it solves

SEO agencies cannot prove causation. They can show traffic went up after a
campaign, but they cannot separate their work from seasonal trends, algorithm
updates, or pure coincidence. Clients know this, and it erodes trust and
justifies churn.

Uplift runs a Bayesian causal impact model (similar to Google's CausalImpact R
package) against GA4 data to produce a statistically rigorous answer to the
question: did this specific intervention cause a measurable lift?

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
│   ├── uplift_core/        # Causal inference engine — no I/O, pure math
│   ├── uplift_connectors/  # GA4 and Google OAuth API clients
│   ├── uplift_db/          # SQLx repositories and database migrations
│   ├── uplift_jobs/        # Apalis background workers
│   ├── uplift_api/         # Axum HTTP server — entry point binary
│   └── uplift_web/         # Leptos SSR frontend
├── docker-compose.yml      # Local Postgres
├── Leptos.toml             # cargo-leptos config
└── Cargo.toml              # Workspace root
```

---

## Prerequisites

- Rust (latest stable) — install via [rustup.rs](https://rustup.rs)
- Docker — for local Postgres
- sqlx-cli — for running migrations

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

Stripe and SMTP are optional for local development — leave them blank.

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
4. Add your email as a test user
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

# Optional
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
Causal model runs — fits a Bayesian structural time series model
    ↓
Results saved: lift %, confidence interval, probability of effect, narrative
    ↓
User views the results page with chart and plain-English explanation
```

---

## Build phases

| Phase | Description | Status |
|---|---|---|
| 1 | Workspace architecture | Done |
| 2 | Google OAuth + token storage | Done |
| 3 | Database layer — SQLx repositories | Done |
| 4 | Background jobs — Apalis workers | Done |
| 5 | GA4 connector — fetch daily metrics | Done |
| 6 | Leptos SSR frontend | Done |
| 7 | Causal inference engine | In progress |
| 8 | Stripe billing | Planned |
| 9 | Marketing site | Planned |

---

## Contributing

This is a private project. Access is by invitation only.
Contact [kimothokarani@gmail.com](mailto:kimothokarani@gmail.com) to request access.

---

## License

Private and confidential. All rights reserved.