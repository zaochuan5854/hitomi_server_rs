fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::env;
    use anyhow::{Result, Context, Ok};
    use sea_orm::{ConnectOptions, ConnectionTrait, Database};

    #[tokio::test]
    async fn test_postgres_connection() -> Result<()> {
        let database_url = env::var("DATABASE_URL").with_context(|| "DATABASE_URL is not set")?;
        let mut opt = ConnectOptions::new(&database_url).to_owned();
        opt.max_connections(5);
        let db = Database::connect(opt).await?;

    let query = sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres, "SELECT 1 AS value".to_owned());

    let query_res = db
        .query_one_raw(query)
        .await?
        .context("Failed to get query result")?;

    // 5. 値の取り出し
    // try_get はカラム名と型を指定して値を取得します
    let value: i32 = query_res.try_get("", "value")?;
        assert_eq!(value, 1);
        println!("PostgreSQL connection test passed with value: {}", value);

        Ok(())
    }
}