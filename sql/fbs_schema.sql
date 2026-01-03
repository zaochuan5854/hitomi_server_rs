-- Compress type table (先に作成する必要がある) --
CREATE TABLE IF NOT EXISTS fbs_compress_types (id SERIAL PRIMARY KEY, name TEXT NOT NULL UNIQUE);

-- FlatBuffers table --
CREATE TABLE IF NOT EXISTS fbs_galleries (
    gallery_id INTEGER PRIMARY KEY, 
    data BYTEA NOT NULL, 
    compress_type INTEGER NOT NULL REFERENCES fbs_compress_types(id)
);

-- BYTAの圧縮を無効化
ALTER TABLE fbs_galleries ALTER COLUMN data SET STORAGE EXTERNAL;

-- 圧縮形式の初期データ挿入
INSERT INTO fbs_compress_types (name) VALUES ('none'), ('zstd') ON CONFLICT DO NOTHING;
