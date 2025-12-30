use hitomi_server_rs::model::Gallery;
use anyhow::{Context,  Result};
use std::path::{Path};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use rayon::prelude::*;

fn main() -> Result<()> {
    let raw_jsonl_dir = Path::new("data/raw_json/");
    let normalized_jsonl_dir = Path::new("data/normalized_json/");

    let raw_jsonl_entries: Vec<_> = std::fs::read_dir(raw_jsonl_dir)?
        .filter_map(|e| e.ok())
        .collect();

    raw_jsonl_entries.par_iter().for_each(|entry| {
        let raw_jsonl = entry.path();
        if raw_jsonl.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            let file_stem = raw_jsonl.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            let normalized_jsonl = normalized_jsonl_dir.join(format!("{}.json", file_stem));

            if let Err(e) = normalize_jsonl(&raw_jsonl, &normalized_jsonl) {
                eprintln!("Error processing {:?}: {:?}", raw_jsonl, e);
            } else {
                println!("Processed {:?}", raw_jsonl);
            }
        }
    });
    println!("Normalization completed.");
    Ok(())
}


fn normalize_jsonl(raw_jsonl_path: &Path, normalized_jsonl_path: &Path) -> Result<()> {
    //! jsonlに含まれる各jsonを正規化して出力
    
    if normalized_jsonl_path.exists() {
        println!("Output file {:?} already exists. Skipping.", normalized_jsonl_path);
        return Ok(());
    }
    let raw_jsonl_file = File::open(raw_jsonl_path)
        .with_context(|| format!("Failed to open input file: {:?}", raw_jsonl_path))?;
    let mut raw_jsonl_reader = BufReader::new(raw_jsonl_file);

    // 一時ファイルに書き込み
    let output_tmp_file = normalized_jsonl_path.with_extension("jsonl.tmp");

    let mut writer = BufWriter::new(File::create(&output_tmp_file)?);
    let mut buf = Vec::with_capacity(10*1024*1024);

    // JSONLファイルを1行ずつ読み込み、正規化して書き込み
    loop {
        buf.clear(); // 次の行を読み込む前に必ずクリアする
        let size = raw_jsonl_reader.read_until(b'\n', &mut buf)?;
        if size == 0 { break; } // 終端に達したら終了

        {
            match serde_json::from_slice::<Gallery>(&mut buf) {
                Ok(value) => {
                    let json_str = serde_json::to_string(&value)?;
                    writer.write_all(json_str.as_bytes())?;
                    writer.write_all(b"\n")?;
                },
                Err(e) => {
                    let json_str = String::from_utf8_lossy(&buf);
                    eprintln!("Failed to parse JSON line as Gallery. line: {:?} error: {:?}", json_str, e);
                }
            }
        }
    }
    writer.flush()?;

    std::fs::rename(output_tmp_file, normalized_jsonl_path)
        .with_context(|| format!("Failed to rename temp file to output file: {:?}", normalized_jsonl_path))?;

    Ok(())
}
