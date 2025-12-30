use hitomi_server_rs::domain::gallery::Gallery;
use hitomi_server_rs::mapper::galleries_mapper;
use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, ConnectionTrait, Statement};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};

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

    // 3. JSONLファイルパスを取得して処理
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let jsonl_path = &args[1];
        println!("Reading from: {}", jsonl_path);
        let path = Path::new(jsonl_path);
        let metadata = std::fs::metadata(path)?;
        let pb = ProgressBar::new(metadata.len());
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        
        import_jsonl_to_db(&db, path, pb.clone()).await?;
        pb.finish_with_message("Import completed");
    } else {
        let dir_path = Path::new("data/normalized_json/");
        let mut entries: Vec<_> = std::fs::read_dir(dir_path)?
            .filter_map(|e| e.ok())
            .collect();

        // ファイル名順にソート
        entries.sort_by_key(|e| e.path());

        // 全ファイルサイズの合計を計算
        let total_size: u64 = entries.iter()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
            .sum();

        // プログレスバーの設定
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));

        // 並列処理のためのセマフォ（同時実行数5）
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5));
        let mut handles = vec![];

        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let db = db.clone();
                let semaphore = semaphore.clone();
                let pb = pb.clone();
                
                let handle = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.context("Failed to acquire semaphore")?;
                    // println!("Start processing: {:?}", path);
                    let result = import_jsonl_to_db(&db, &path, pb).await;
                    // println!("Finished processing: {:?}", path);
                    result
                });
                handles.push(handle);
            }
        }

        // 全タスクの完了を待機
        for handle in handles {
            handle.await.context("Task join failed")??;
        }
        pb.finish_with_message("All imports completed");
    }

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
    pb: ProgressBar,
) -> Result<()> {
    let file = File::open(jsonl_path)
        .with_context(|| format!("Failed to open file: {:?}", jsonl_path))?;
    
    // プログレスバーでラップ
    let reader = BufReader::new(pb.wrap_read(file));

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
                        continue;
                    }
                    Err(e) => {
                        pb.println(format!("Failed to insert gallery at line {}: {}", line_number, e));
                    }
                }
            }
            Err(e) => {
                pb.println(format!("Failed to parse JSON at line {}: {}", line_number, e));
            }
        }
    }

    // 個別のファイル完了ログも削除または必要なら pb.println で出力
    // pb.println(format!("Finished: {:?} (Success: {}, Failed: {})", jsonl_path, success_count, error_count));

    Ok(())
}
