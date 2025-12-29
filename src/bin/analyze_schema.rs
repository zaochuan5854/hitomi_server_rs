use genson_rs::SchemaBuilder;
use anyhow::{Context,  Result};
use std::path::{Path};
use std::fs::{File, read_to_string};
use std::io::{BufRead, BufReader, BufWriter, Write};
use rayon::prelude::*;

fn main() -> Result<()> {
    let raw_jsonl = Path::new("data/raw_json/");
    let schema_output = Path::new("data/schema/");
    let merged_schema_output = Path::new("data/merged_schema.json");

    let jsonl_entries: Vec<_> = std::fs::read_dir(raw_jsonl)?
        .filter_map(|e| e.ok())
        .collect();

    jsonl_entries.par_iter().for_each(|entry| {
        let jsonl = entry.path();
        if jsonl.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            let file_stem = jsonl.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let output_schema = schema_output.join(format!("{}.json", file_stem));

            if let Err(e) = jsonl_to_schema(&jsonl, &output_schema) {
                eprintln!("Error processing {:?}: {:?}", jsonl, e);
            } else {
                println!("Processed {:?}", jsonl);
            }
        }
    });
    merge_schemas(schema_output, merged_schema_output)?;
    println!("Schema analysis completed.");
    Ok(())
}


fn jsonl_to_schema(jsonl_path: &Path, schema_path: &Path) -> Result<()> {
    //! jsonlに含まれる各jsonのスキーマを分析
    
    if schema_path.exists() {
        println!("Output file {:?} already exists. Skipping.", schema_path);
        return Ok(());
    }
    let jsonl_file = File::open(jsonl_path)
        .with_context(|| format!("Failed to open input file: {:?}", jsonl_path))?;
    let mut jsonl_reader = BufReader::new(jsonl_file);

    // 一時ファイルに書き込み
    let output_tmp_file = schema_path.with_extension("jsonl.tmp");

    let mut schema_writer = BufWriter::new(File::create(&output_tmp_file)?);
    let mut schema_builder = SchemaBuilder::new(Some("AUTO"));
    let mut buf = Vec::with_capacity(10*1024*1024);

    // JSONLファイルを1行ずつ読み込み、スキーマに追加
    loop {
        buf.clear(); // 次の行を読み込む前に必ずクリアする
        let size = jsonl_reader.read_until(b'\n', &mut buf)?;
        if size == 0 { break; } // 終端に達したら終了

        {
            match simd_json::to_borrowed_value(&mut buf) {
                Ok(value) => {
                    schema_builder.add_object(&value);
                },
                Err(e) => {
                    eprintln!("Failed to parse JSON line in {:?}: {:?}", jsonl_path, e);
                }
            }
        }
    }
    let schema_json_str = schema_builder.to_json();
    schema_writer.write_all(schema_json_str.as_bytes())?;
    schema_writer.flush()?;

    std::fs::rename(output_tmp_file, schema_path)
        .with_context(|| format!("Failed to rename temp file to output file: {:?}", schema_path))?;

    Ok(())
}

fn merge_schemas(schema_dir: &Path, merged_schema_path: &Path) -> Result<()> {
    //! 複数のスキーマファイルをマージして1つのスキーマファイルにまとめる
    
    if merged_schema_path.exists() {
        println!("Merged schema file {:?} already exists. Skipping.", merged_schema_path);
        return Ok(());
    }

    let mut merged_schema_builder = SchemaBuilder::new(Some("AUTO"));

    let entries: Vec<_> = std::fs::read_dir(schema_dir)?
        .filter_map(|e| e.ok())
        .collect();

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {

            let json_str = read_to_string(&path)
                .with_context(|| format!("Failed to read schema file: {:?}", path))?;
            let schema = serde_json::from_str::<serde_json::Value>(&json_str)
                .with_context(|| format!("Failed to parse schema JSON in file: {:?}", path))?;
            merged_schema_builder.add_schema(schema);

        }
    }

    let merged_schema_json_str = merged_schema_builder.to_json();
    let mut merged_schema_writer = BufWriter::new(File::create(merged_schema_path)?);
    merged_schema_writer.write_all(merged_schema_json_str.as_bytes())?;
    merged_schema_writer.flush()?;

    Ok(())
}