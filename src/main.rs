fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::env;
    use anyhow::{Result, Context, Ok};
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn test_postgres_connection() -> Result<()> {
        let database_url = env::var("DATABASE_URL").with_context(|| "DATABASE_URL is not set")?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;
        
        let row = sqlx::query!("SELECT 1 AS value")
            .fetch_one(&pool)
            .await?;
        
        let value = row.value.ok_or_else(|| anyhow::anyhow!("Fail to get value"))?;
        assert_eq!(value, 1);
        println!("PostgreSQL connection test passed with value: {}", value);

        Ok(())
    }
}