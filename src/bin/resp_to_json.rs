use serde::{Serialize, Deserialize};
use anyhow::{Context,  Result};
use std::path::{Path};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

#[derive(Serialize, Deserialize, Debug)]
struct RespRecord {
    gallery_id: i32,
    status: i32,
    raw_data: String,
    meta_data: String,
}

fn main() -> Result<()> {
    let resp_data_dir = Path::new("data/resp_json/");
    let jsonl_output_path = Path::new("data/raw_json/");

    assert!(resp_data_dir.exists(), "RESP data directory does not exist");
    if !jsonl_output_path.exists() {
        std::fs::create_dir_all(jsonl_output_path)?;
    }

    let entries: Vec<_> = std::fs::read_dir(resp_data_dir)?
        .filter_map(|e| e.ok())
        .collect();

    entries.iter().for_each(|entry| {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let output_file_path = jsonl_output_path.join(format!("{}.jsonl", file_stem));

            if let Err(e) = process_resp_file(&path, &output_file_path) {
                eprintln!("Error processing {:?}: {:?}", path, e);
            } else {
                println!("Processed {:?}", path);
            }
        }
    });

    Ok(())
}


fn process_resp_file(input_path: &Path, output_path: &Path) -> Result<()> {
    //! RESPファイルを読み込み、JSONL形式で出力する
    //! 既に出力ファイルが存在する場合はスキップする
    
    if output_path.exists() {
        println!("Output file {:?} already exists. Skipping.", output_path);
        return Ok(());
    }
    let input_file = File::open(input_path)
        .with_context(|| format!("Failed to open input file: {:?}", input_path))?;
    let reader = BufReader::new(input_file);

    // 一時ファイルに書き込み
    let output_tmp_file = output_path.with_extension("jsonl.tmp");

    let mut writer = BufWriter::new(File::create(&output_tmp_file)?);

    for line in reader.lines() {
        let line = line?;
        write_jsonl(&line, &mut writer).inspect_err(|e| eprintln!("An error occurred while processing line: {}: {:?}", line, e)).ok();
    }
    writer.flush()?;

    // 一時ファイルを最終出力ファイルにリネーム
    std::fs::rename(output_tmp_file, output_path)
        .with_context(|| format!("Failed to rename temp file to output file: {:?}", output_path))?;

    Ok(())
}

fn write_jsonl(resp: &str, writer: &mut BufWriter<File>) -> Result<()> {
    //! RESPデータをパースしてJSONL形式で書き出す
    let record: RespRecord = serde_json::from_str(&resp)
        .with_context(|| "Failed to parse RESP data")?;
    
    if record.status != 200 {
        if record.status == 404 {
            // 404は多いので無視
            return Ok(());
        }
        else {
            anyhow::bail!("Unexpected status code: {}, metadata: {}", record.status, record.meta_data);
        }
    }

    // raw_dataからJSON部分を抽出
    let json_prefix = "var galleryinfo = ";
    let json_str = record.raw_data.strip_prefix(json_prefix)
    .ok_or_else(|| anyhow::anyhow!("Unexpected prefix gallery_id: {} raw_data: {}", record.gallery_id, record.raw_data))?;

    let mut raw_json = serde_json::from_str::<serde_json::Value>(json_str)
        .with_context(|| format!("Failed to parse raw_data as JSON for gallery_id: {}", record.gallery_id))?;

    // gallery_idを追加
    let obj = raw_json.as_object_mut()
    .context(anyhow::anyhow!("Expected JSON object for gallery_id: {}", record.gallery_id))?;
    obj.insert("gallery_id".to_string(), serde_json::json!(record.gallery_id));

    // JSONL形式で書き出し
    serde_json::to_writer(&mut *writer, obj)
        .with_context(|| format!("Failed to serialize JSON for gallery_id: {}", record.gallery_id))?;
    writer.write_all(b"\n")?;
    

    Ok(())
}