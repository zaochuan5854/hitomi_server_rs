use hitomi_server_rs::domain::gallery::Gallery;
use hitomi_server_rs::mapper::galleries_mapper;
use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, ConnectionTrait, Statement};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. DATABASE_URL を取得
    let database_url = env::var("DATABASE_URL")
        .with_context(|| "DATABASE_URL is not set")?;

    // 2. DB接続
    let mut opt = ConnectOptions::new(&database_url).to_owned();
    opt.max_connections(10);
    let db = Database::connect(opt).await
        .context("Failed to connect to database")?;

    println!("Connected to database");

    // 2.5. テーブル作成
    println!("Creating tables...");
    create_tables(&db).await?;
    println!("Tables created successfully");

    // 3. JSONLファイルパスを取得（引数またはデフォルト）
    let args: Vec<String> = env::args().collect();
    let jsonl_path = if args.len() > 1 {
        &args[1]
    } else {
        "data/normalized_json/000000001-000100000.json"
    };

    println!("Reading from: {}", jsonl_path);

    // 4. JSONLファイルを読み込んでDBに保存
    import_jsonl_to_db(&db, Path::new(jsonl_path)).await?;

    println!("Import completed successfully");

    Ok(())
}

async fn create_tables(db: &sea_orm::DatabaseConnection) -> Result<()> {
    let schema = include_str!("../../sql/schema.sql");
    let statements: Vec<&str> = schema
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for sql in statements {
        db.execute_raw(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            sql.to_string(),
        ))
        .await
        .with_context(|| format!("Failed to execute: {}", sql))?;
    }

    Ok(())
}

async fn import_jsonl_to_db(
    db: &sea_orm::DatabaseConnection,
    jsonl_path: &Path,
) -> Result<()> {
    let file = File::open(jsonl_path)
        .with_context(|| format!("Failed to open file: {:?}", jsonl_path))?;
    let reader = BufReader::new(file);

    let mut success_count = 0;
    let mut error_count = 0;
    let mut line_number = 0;

    for line in reader.lines() {
        line_number += 1;
        let line = line.context("Failed to read line")?;
        
        // 空行はスキップ
        if line.trim().is_empty() {
            continue;
        }

        // JSONをパース
        match serde_json::from_str::<Gallery>(&line) {
            Ok(gallery) => {
                // DBに保存
                match galleries_mapper::insert_gallery(db, gallery).await {
                    Ok(_) => {
                        success_count += 1;
                        if success_count % 100 == 0 {
                            println!("Imported {} galleries...", success_count);
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        eprintln!("Failed to insert gallery at line {}: {}", line_number, e);
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("Failed to parse JSON at line {}: {}", line_number, e);
            }
        }
    }

    println!("\nImport summary:");
    println!("  Successfully imported: {}", success_count);
    println!("  Failed: {}", error_count);
    println!("  Total lines processed: {}", line_number);

    Ok(())
}
