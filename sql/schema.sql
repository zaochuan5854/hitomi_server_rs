-- Languages table
CREATE TABLE IF NOT EXISTS languages (id SERIAL PRIMARY KEY, name TEXT NOT NULL UNIQUE, local_name TEXT, url TEXT);
CREATE INDEX IF NOT EXISTS idx_languages_name ON languages(name);

-- Galleries table
CREATE TABLE IF NOT EXISTS galleries (id SERIAL PRIMARY KEY, gallery_id INTEGER NOT NULL UNIQUE, title TEXT NOT NULL, date TEXT NOT NULL, type TEXT NOT NULL, external_id TEXT NOT NULL, scene_indexes INTEGER[] NOT NULL DEFAULT '{}', related_ids TEXT[] NOT NULL DEFAULT '{}', japanese_title TEXT, language_id INTEGER REFERENCES languages(id), translation_group_id TEXT[] NOT NULL DEFAULT '{}', video TEXT, videofilename TEXT, gallery_url TEXT, date_published TEXT, blocked BOOLEAN NOT NULL DEFAULT FALSE, files JSONB NOT NULL DEFAULT '[]');
CREATE INDEX IF NOT EXISTS idx_galleries_gallery_id ON galleries(gallery_id);
CREATE INDEX IF NOT EXISTS idx_galleries_language_id ON galleries(language_id);

-- Tags table
CREATE TABLE IF NOT EXISTS tags (id SERIAL PRIMARY KEY, name TEXT NOT NULL, url TEXT NOT NULL, male BOOLEAN NOT NULL DEFAULT FALSE, female BOOLEAN NOT NULL DEFAULT FALSE, UNIQUE(name, male, female));
CREATE INDEX IF NOT EXISTS idx_tags_name_male_female ON tags(name, male, female);

-- Artists table
CREATE TABLE IF NOT EXISTS artists (id SERIAL PRIMARY KEY, artist TEXT NOT NULL UNIQUE, url TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_artists_artist ON artists(artist);

-- Groups table
CREATE TABLE IF NOT EXISTS groups (id SERIAL PRIMARY KEY, "group" TEXT NOT NULL UNIQUE, url TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_groups_group ON groups("group");

-- Characters table
CREATE TABLE IF NOT EXISTS characters (id SERIAL PRIMARY KEY, character TEXT NOT NULL UNIQUE, url TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_characters_character ON characters(character);

-- Parodies table
CREATE TABLE IF NOT EXISTS parodies (id SERIAL PRIMARY KEY, parody TEXT NOT NULL UNIQUE, url TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_parodies_parody ON parodies(parody);

-- Junction tables
CREATE TABLE IF NOT EXISTS gallery_tags (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, tag_id));
CREATE INDEX IF NOT EXISTS idx_gallery_tags_gallery_id ON gallery_tags(gallery_id);
CREATE INDEX IF NOT EXISTS idx_gallery_tags_tag_id ON gallery_tags(tag_id);

CREATE TABLE IF NOT EXISTS gallery_artists (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, artist_id));
CREATE INDEX IF NOT EXISTS idx_gallery_artists_gallery_id ON gallery_artists(gallery_id);
CREATE INDEX IF NOT EXISTS idx_gallery_artists_artist_id ON gallery_artists(artist_id);

CREATE TABLE IF NOT EXISTS gallery_groups (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, group_id));
CREATE INDEX IF NOT EXISTS idx_gallery_groups_gallery_id ON gallery_groups(gallery_id);
CREATE INDEX IF NOT EXISTS idx_gallery_groups_group_id ON gallery_groups(group_id);

CREATE TABLE IF NOT EXISTS gallery_characters (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, character_id INTEGER NOT NULL REFERENCES characters(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, character_id));
CREATE INDEX IF NOT EXISTS idx_gallery_characters_gallery_id ON gallery_characters(gallery_id);
CREATE INDEX IF NOT EXISTS idx_gallery_characters_character_id ON gallery_characters(character_id);

CREATE TABLE IF NOT EXISTS gallery_parodies (gallery_id INTEGER NOT NULL REFERENCES galleries(id) ON DELETE CASCADE, parody_id INTEGER NOT NULL REFERENCES parodies(id) ON DELETE CASCADE, PRIMARY KEY (gallery_id, parody_id));
CREATE INDEX IF NOT EXISTS idx_gallery_parodies_gallery_id ON gallery_parodies(gallery_id);
CREATE INDEX IF NOT EXISTS idx_gallery_parodies_parody_id ON gallery_parodies(parody_id);
