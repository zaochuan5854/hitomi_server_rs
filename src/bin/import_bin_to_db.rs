use hitomi_server_rs::domain::gallery::Gallery;
use hitomi_server_rs::fbs::converter;
use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, ConnectionTrait, Statement};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};

const BATCH_SIZE: usize = 500;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. DATABASE_URL_BINARY を取得
    let database_url = env::var("DATABASE_URL_BINARY")
        .with_context(|| "DATABASE_URL_BINARY is not set")?;

    // 2. DB接続
    let mut opt = ConnectOptions::new(&database_url).to_owned();
    opt.max_connections(20);
    opt.connect_timeout(std::time::Duration::from_secs(10));
    opt.acquire_timeout(std::time::Duration::from_secs(10));
    opt.set_schema_search_path("public");
    let db = Database::connect(opt).await
        .context("Failed to connect to database")?;

    println!("Connected to database");

    // 2.4. テーブル削除
    println!("Dropping tables...");
    drop_tables(&db).await?;

    // 2.5. テーブル作成
    println!("Creating tables...");
    create_tables(&db).await?;
    println!("Tables created successfully");

    // 2.6. compress_typeのIDを取得または作成
    let compress_type_id = get_or_create_compress_type(&db, "zstd").await?;
    println!("Using compress_type_id: {} for zstd", compress_type_id);

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
        
        import_jsonl_to_fbs_db(&db, path, pb.clone(), compress_type_id).await?;
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

        // 順次処理
        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                import_jsonl_to_fbs_db(&db, &path, pb.clone(), compress_type_id).await?;
            }
        }
        
        pb.finish_with_message("All imports completed");
    }

    println!("Import completed successfully");

    Ok(())
}

async fn create_tables(db: &sea_orm::DatabaseConnection) -> Result<()> {
    let schema = include_str!("../../sql/fbs_schema.sql");
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

async fn drop_tables(db: &sea_orm::DatabaseConnection) -> Result<()> {
    // 外部キー制約のため、参照する側(fbs_galleries)を先にdrop
    let statements = vec![
        "DROP TABLE IF EXISTS fbs_galleries CASCADE",
        "DROP TABLE IF EXISTS fbs_compress_types CASCADE",
    ];

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

async fn get_or_create_compress_type(
    db: &sea_orm::DatabaseConnection,
    name: &str,
) -> Result<i32> {
    // まず既存のIDを検索
    let query = Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT id FROM fbs_compress_types WHERE name = $1",
        vec![name.into()],
    );

    if let Some(row) = db.query_one_raw(query).await? {
        let id: i32 = row.try_get("", "id")?;
        return Ok(id);
    }

    // 存在しない場合は挿入
    let insert_query = Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "INSERT INTO fbs_compress_types (name) VALUES ($1) RETURNING id",
        vec![name.into()],
    );

    let row = db.query_one_raw(insert_query)
        .await?
        .with_context(|| format!("Failed to insert compress_type: {}", name))?;

    let id: i32 = row.try_get("", "id")?;
    Ok(id)
}

async fn import_jsonl_to_fbs_db(
    db: &sea_orm::DatabaseConnection,
    jsonl_path: &Path,
    pb: ProgressBar,
    compress_type_id: i32,
) -> Result<()> {
    // 1. moveのためにclone/to_path_buf
    let path_buf = jsonl_path.to_path_buf();
    let db_clone = db.clone();
    let pb_clone = pb.clone();

    // 2. チャンネルの作成（バッファを持たせて流量調整）
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<(i32, Vec<u8>, i32)>>(5);

    // 3. 読み込み・パース・FBS変換用のタスクを分離して実行 (Producer)
    let reader_handle = tokio::spawn(async move {
        let mut chunk = Vec::with_capacity(BATCH_SIZE);
        let file = File::open(&path_buf)
            .with_context(|| format!("Failed to open file: {:?}", path_buf))?;
        let reader = BufReader::new(pb_clone.wrap_read(file));

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    pb_clone.println(format!("Failed to read line: {}", e));
                    continue;
                }
            };

            // 空行はスキップ
            if line.trim().is_empty() {
                continue;
            }

            // JSONをパース
            let gallery: Gallery = match serde_json::from_str(&line) {
                Ok(g) => g,
                Err(e) => {
                    pb_clone.println(format!("Failed to parse JSON: {}", e));
                    continue;
                }
            };

            let gallery_id = gallery.gallery_id;

            // FlatBuffersに変換
            let fbs_data = converter::serialize_gallery(&gallery);

            // zstdで圧縮
            let compressed_data = match zstd::encode_all(&fbs_data[..], 3) {
                Ok(data) => data,
                Err(e) => {
                    pb_clone.println(format!("Failed to compress gallery {}: {}", gallery_id, e));
                    continue;
                }
            };

            chunk.push((gallery_id, compressed_data, compress_type_id));

            if chunk.len() >= BATCH_SIZE {
                // DBタスクへ送信
                if tx.send(std::mem::take(&mut chunk)).await.is_err() {
                    break;
                }
            }
        }
        if !chunk.is_empty() { 
            tx.send(chunk).await.ok(); 
        }
        Ok::<(), anyhow::Error>(())
    });

    // 4. メインタスクでDBインサートをひたすら実行 (Consumer)
    while let Some(batch) = rx.recv().await {
        if let Err(e) = insert_fbs_batch(&db_clone, batch).await {
            pb.println(format!("Insert error: {:?}", e));
        }
    }

    reader_handle.await??; // 読み込み完了を待機
    Ok(())
}

async fn insert_fbs_batch(
    db: &sea_orm::DatabaseConnection,
    batch: Vec<(i32, Vec<u8>, i32)>,
) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    // バッチインサート用のSQLを構築
    let mut values_parts = Vec::with_capacity(batch.len());
    let mut params: Vec<sea_orm::Value> = Vec::with_capacity(batch.len() * 3);

    for (idx, (gallery_id, compressed_data, compress_type_id)) in batch.into_iter().enumerate() {
        let param_idx = idx * 3;
        values_parts.push(format!("(${}, ${}, ${})", param_idx + 1, param_idx + 2, param_idx + 3));
        
        params.push(gallery_id.into());
        params.push(compressed_data.into());
        params.push(compress_type_id.into());
    }

    let sql = format!(
        "INSERT INTO fbs_galleries (gallery_id, data, compress_type) VALUES {} ON CONFLICT (gallery_id) DO NOTHING",
        values_parts.join(", ")
    );

    db.execute_raw(Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        &sql,
        params,
    ))
    .await
    .context("Failed to insert FBS batch")?;

    Ok(())
}
