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
    let statements = vec![
        // Languages table
        "CREATE TABLE IF NOT EXISTS languages (id SERIAL PRIMARY KEY, name TEXT NOT NULL, local_name TEXT, url TEXT)",
        "CREATE INDEX IF NOT EXISTS idx_languages_name ON languages(name)",
        
        // Galleries table
        "CREATE TABLE IF NOT EXISTS galleries (id SERIAL PRIMARY KEY, gallery_id INTEGER NOT NULL UNIQUE, title TEXT NOT NULL, date TEXT NOT NULL, type TEXT NOT NULL, external_id TEXT NOT NULL, scene_indexes INTEGER[] NOT NULL DEFAULT '{}', related_ids TEXT[] NOT NULL DEFAULT '{}', japanese_title TEXT, language_id INTEGER REFERENCES languages(id), translation_group_id TEXT[] NOT NULL DEFAULT '{}', video TEXT, videofilename TEXT, gallery_url TEXT, date_published TEXT, blocked BOOLEAN NOT NULL DEFAULT FALSE)",
        "CREATE INDEX IF NOT EXISTS idx_galleries_gallery_id ON galleries(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_galleries_language_id ON galleries(language_id)",
        
        // Tags table
        "CREATE TABLE IF NOT EXISTS tags (id SERIAL PRIMARY KEY, name TEXT NOT NULL, url TEXT NOT NULL, male BOOLEAN NOT NULL DEFAULT FALSE, female BOOLEAN NOT NULL DEFAULT FALSE)",
        "CREATE INDEX IF NOT EXISTS idx_tags_name_male_female ON tags(name, male, female)",
        
        // Artists table
        "CREATE TABLE IF NOT EXISTS artists (id SERIAL PRIMARY KEY, artist TEXT NOT NULL, url TEXT NOT NULL)",
        "CREATE INDEX IF NOT EXISTS idx_artists_artist ON artists(artist)",
        
        // Groups table
        "CREATE TABLE IF NOT EXISTS groups (id SERIAL PRIMARY KEY, \"group\" TEXT NOT NULL, url TEXT NOT NULL)",
        "CREATE INDEX IF NOT EXISTS idx_groups_group ON groups(\"group\")",
        
        // Characters table
        "CREATE TABLE IF NOT EXISTS characters (id SERIAL PRIMARY KEY, character TEXT NOT NULL, url TEXT NOT NULL)",
        "CREATE INDEX IF NOT EXISTS idx_characters_character ON characters(character)",
        
        // Parodies table
        "CREATE TABLE IF NOT EXISTS parodies (id SERIAL PRIMARY KEY, parody TEXT NOT NULL, url TEXT NOT NULL)",
        "CREATE INDEX IF NOT EXISTS idx_parodies_parody ON parodies(parody)",
        
        // Files table
        "CREATE TABLE IF NOT EXISTS files (id SERIAL PRIMARY KEY, name TEXT NOT NULL, hash TEXT NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL, hasavif BOOLEAN NOT NULL DEFAULT FALSE, haswebp BOOLEAN NOT NULL DEFAULT FALSE, hasjxl BOOLEAN NOT NULL DEFAULT FALSE, single BOOLEAN NOT NULL DEFAULT FALSE)",
        "CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)",
        
        // Junction tables
        "CREATE TABLE IF NOT EXISTS gallery_tags (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, tag_id))",
        "CREATE INDEX IF NOT EXISTS idx_gallery_tags_gallery_id ON gallery_tags(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_gallery_tags_tag_id ON gallery_tags(tag_id)",
        
        "CREATE TABLE IF NOT EXISTS gallery_artists (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, artist_id))",
        "CREATE INDEX IF NOT EXISTS idx_gallery_artists_gallery_id ON gallery_artists(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_gallery_artists_artist_id ON gallery_artists(artist_id)",
        
        "CREATE TABLE IF NOT EXISTS gallery_groups (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, group_id))",
        "CREATE INDEX IF NOT EXISTS idx_gallery_groups_gallery_id ON gallery_groups(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_gallery_groups_group_id ON gallery_groups(group_id)",
        
        "CREATE TABLE IF NOT EXISTS gallery_characters (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, character_id))",
        "CREATE INDEX IF NOT EXISTS idx_gallery_characters_gallery_id ON gallery_characters(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_gallery_characters_character_id ON gallery_characters(character_id)",
        
        "CREATE TABLE IF NOT EXISTS gallery_parodies (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, parody_id INTEGER NOT NULL REFERENCES parodies(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, parody_id))",
        "CREATE INDEX IF NOT EXISTS idx_gallery_parodies_gallery_id ON gallery_parodies(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_gallery_parodies_parody_id ON gallery_parodies(parody_id)",
        
        "CREATE TABLE IF NOT EXISTS gallery_files (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, file_id))",
        "CREATE INDEX IF NOT EXISTS idx_gallery_files_gallery_id ON gallery_files(gallery_id)",
        "CREATE INDEX IF NOT EXISTS idx_gallery_files_file_id ON gallery_files(file_id)",
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
