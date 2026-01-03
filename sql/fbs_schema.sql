-- Compress type table (先に作成する必要がある) --
CREATE TABLE IF NOT EXISTS fbs_compress_types (id SERIAL PRIMARY KEY, name TEXT NOT NULL UNIQUE);

-- FlatBuffers table --
CREATE TABLE IF NOT EXISTS fbs_galleries (
    gallery_id INTEGER PRIMARY KEY, 
    data BYTEA NOT NULL, 
    compress_type INTEGER NOT NULL REFERENCES fbs_compress_types(id)
);
