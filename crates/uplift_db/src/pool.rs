use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn connect(database_url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> sqlx::Result<(), sqlx::migrate::MigrateError> {
    let mut migrator = sqlx::migrate!("./migrations");
    // Apalis shares _sqlx_migrations — don't fail if its entries are in the table
    migrator.ignore_missing = true;
    // Disable advisory lock so restarts don't hang on a crashed previous run
    migrator.locking = false;
    migrator.run(pool).await
}
