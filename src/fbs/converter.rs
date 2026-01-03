use crate::fbs::gallery_generated::gallery::schema;
use crate::domain::gallery;
use flatbuffers::{FlatBufferBuilder, WIPOffset};

pub fn serialize_gallery(gallery_data: &gallery::Gallery) -> Vec<u8> {
    let mut fbb = FlatBufferBuilder::with_capacity(1024*50); // 50KB 初期容量
    let root_offset = gallery_data.to_flatbuffer(&mut fbb);
    fbb.finish(root_offset, None);

    return fbb.finished_data().to_vec()
}

pub fn deserialize_gallery(data: &[u8]) -> gallery::Gallery {
    
    match schema::root_as_gallery(data) {
        Ok(data) => {
            let files: Option<Vec<gallery::File>> = data.files().map(|fs| {
            fs.iter()
            .map(|f| gallery::File {
                name: f.name().to_string(),
                hash: f.hash().to_string(),
                width: f.width(),
                height: f.height(),
                hasavif: f.hasavif(),
                haswebp: f.haswebp(),
                hasjxl: f.hasjxl(),
                single: f.single(),
            })
            .collect() // ここで Vec<gallery::File> になる
            
            });
            let languages: Option<Vec<gallery::Language>> = data.languages().map(|ls| {
                ls.iter()
                .map(|l| gallery::Language {
                    name: l.name().to_string(),
                    language_localname: l.language_localname().to_string(),
                    url: l.url().to_string(),
                    galleryid: l.galleryid().to_string(),
                })
                .collect()
            });
            let scene_indexes = data.scene_indexes().map(|v| v.iter().collect());
            let related = data.related().map(|v| v.iter().map(|s| s.to_string()).collect());
            let artist  = data.artists().map(|as_| {
                as_.iter()
                .map(|a| gallery::Artist {
                    artist: a.artist().to_string(),
                    url: a.url().to_string(),
                })
                .collect()
            });
            let groups = data.groups().map(|gs| {
                gs.iter()
                .map(|g| gallery::Group {
                    group: g.group().to_string(),
                    url: g.url().to_string(),
                })
                .collect()
            });
            let characters = data.characters().map(|cs| {
                cs.iter()
                .map(|c| gallery::Character {
                    character: c.character().to_string(),
                    url: c.url().to_string(),
                })
                .collect()
            });
            let parodies = data.parodies().map(|ps| {
                ps.iter()
                .map(|p| gallery::Parody {
                    parody: p.parody().to_string(),
                    url: p.url().to_string(),
                })
                .collect()
            });
            let tags = data.tags().map(|ts| {
                ts.iter()
                .map(|t| gallery::Tag {
                    tag: t.tag().to_string(),
                    url: t.url().to_string(),
                    male: t.male(),
                    female: t.female(),
                })
                .collect()
            });

            let gallery = gallery::Gallery {
                gallery_id: data.gallery_id(),
                title: data.title().to_string(),
                date: chrono::DateTime::parse_from_rfc3339(data.date()).unwrap(),
                files: files.unwrap_or(vec![]),
                languages: languages.unwrap_or(vec![]),
                scene_indexes: scene_indexes.unwrap_or(vec![]),
                type_: data.type_().to_string(),
                id: data.id().to_string(),
                related: related.unwrap_or(vec![]),
                japanese_title: data.japanese_title().map(|s| s.to_string()),
                language: data.language().map(|s| s.to_string()),
                language_localname: data.language_localname().map(|s| s.to_string()),
                language_url: data.language_url().map(|s| s.to_string()),
                video: data.video().map(|s| s.to_string()),
                videofilename: data.videofilename().map(|s| s.to_string()),
                artists: artist.unwrap_or(vec![]),
                groups: groups.unwrap_or(vec![]),
                characters: characters.unwrap_or(vec![]),
                parodies: parodies.unwrap_or(vec![]),
                tags: tags.unwrap_or(vec![]),
                gallery_url: data.gallery_url().map(|s| s.to_string()),
                date_published: data.date_published().and_then(|date_str| chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()),
                blocked: data.blocked(),
            };
            return gallery
        }
        Err(e) => {
                panic!("Failed to deserialize gallery: {:?}", e);
        }
    }
}

pub trait ToFlatBuffer<'a, T> {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<T>;
}

// Gallery
impl<'a> ToFlatBuffer<'a, schema::Gallery<'a>> for gallery::Gallery {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Gallery<'a>> {
        let title = fbb.create_string(&self.title);
        let data = fbb.create_string(&self.date.to_rfc3339());
        let files = {
            let file_offsets: Vec<WIPOffset<schema::File>> = self
                .files
                .iter()
                .map(|file| file.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&file_offsets)
        };
        let languages = {
            let lang_offsets: Vec<WIPOffset<schema::Language>> = self
                .languages
                .iter()
                .map(|lang| lang.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&lang_offsets)
        };
        let scene_indexes = fbb.create_vector(&self.scene_indexes);
        let type_ = fbb.create_string(&self.type_);
        let id = fbb.create_string(&self.id);
        let related = {
            let related_offsets: Vec<WIPOffset<&str>> = self
                .related
                .iter()
                .map(|rel| fbb.create_string(rel))
                .collect();
            fbb.create_vector(&related_offsets)
        };
        let japanese_title = self
            .japanese_title
            .as_ref()
            .map(|s| fbb.create_string(s));
        let language = self.language.as_ref().map(|s| fbb.create_string(s));
        let language_localname = self
            .language_localname
            .as_ref()
            .map(|s| fbb.create_string(s));
        let language_url = self.language_url.as_ref().map(|s| fbb.create_string(s));
        let video = self.video.as_ref().map(|s| fbb.create_string(s));
        let videofilename = self.videofilename.as_ref().map(|s| fbb.create_string(s));
        let artists = {
            let artist_offsets: Vec<WIPOffset<schema::Artist>> = self
                .artists
                .iter()
                .map(|artist| artist.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&artist_offsets)
        };
        let groups = {
            let group_offsets: Vec<WIPOffset<schema::Group>> = self
                .groups
                .iter()
                .map(|group| group.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&group_offsets)
        };
        let characters = {
            let character_offsets: Vec<WIPOffset<schema::Character>> = self
                .characters
                .iter()
                .map(|character| character.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&character_offsets)
        };
        let parodies = {
            let parody_offsets: Vec<WIPOffset<schema::Parody>> = self
                .parodies
                .iter()
                .map(|parody| parody.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&parody_offsets)
        };
        let tags = {
            let tag_offsets: Vec<WIPOffset<schema::Tag>> = self
                .tags
                .iter()
                .map(|tag| tag.to_flatbuffer(fbb))
                .collect();
            fbb.create_vector(&tag_offsets)
        };
        let gallery_url = self.gallery_url.as_ref().map(|s| fbb.create_string(s));
        let date_published = self
            .date_published
            .as_ref()
            .map(|d| fbb.create_string(&d.to_string()));    
        schema::Gallery::create(fbb, &schema::GalleryArgs {
            gallery_id: self.gallery_id,
            title: Some(title),
            date: Some(data),
            files: Some(files),
            languages: Some(languages),
            scene_indexes: Some(scene_indexes),
            type_: Some(type_),
            id: Some(id),
            related: Some(related),
            japanese_title,
            language,
            language_localname,
            language_url,
            video,
            videofilename,
            artists: Some(artists),
            groups: Some(groups),
            characters: Some(characters),
            parodies: Some(parodies),
            tags: Some(tags),
            gallery_url,
            date_published,
            blocked: self.blocked,
        })
    }
}

// Language
impl<'a> ToFlatBuffer<'a, schema::Language<'a>> for gallery::Language {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Language<'a>> {
        let name = fbb.create_string(&self.name);
        let language_localname = fbb.create_string(&self.language_localname);
        let url = fbb.create_string(&self.url);

        let galleryid = fbb.create_string(&self.galleryid);

        let mut b = schema::LanguageBuilder::new(fbb);
        b.add_name(name);
        b.add_language_localname(language_localname);
        b.add_url(url);
        b.add_galleryid(galleryid);
        b.finish()
    }
}

// Artist
impl<'a> ToFlatBuffer<'a, schema::Artist<'a>> for gallery::Artist {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Artist<'a>> {
        let artist = fbb.create_string(&self.artist);
        let url = fbb.create_string(&self.url);

        let mut b = schema::ArtistBuilder::new(fbb);
        b.add_artist(artist);
        b.add_url(url);
        b.finish()
    }
}

// Group
impl<'a> ToFlatBuffer<'a, schema::Group<'a>> for gallery::Group {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Group<'a>> {
        let group = fbb.create_string(&self.group);
        let url = fbb.create_string(&self.url);
        let mut b = schema::GroupBuilder::new(fbb);
        b.add_group(group);
        b.add_url(url);
        b.finish()
    }
}

// Character
impl<'a> ToFlatBuffer<'a, schema::Character<'a>> for gallery::Character {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Character<'a>> {
        let character = fbb.create_string(&self.character);
        let url = fbb.create_string(&self.url);
        let mut b = schema::CharacterBuilder::new(fbb);
        b.add_character(character);
        b.add_url(url);
        b.finish()
    }
}

// Parody
impl<'a> ToFlatBuffer<'a, schema::Parody<'a>> for gallery::Parody {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Parody<'a>> {
        let parody = fbb.create_string(&self.parody);
        let url = fbb.create_string(&self.url);
        let mut b = schema::ParodyBuilder::new(fbb);
        b.add_parody(parody);
        b.add_url(url);
        b.finish()
    }
}

// Tag
impl<'a> ToFlatBuffer<'a, schema::Tag<'a>> for gallery::Tag {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::Tag<'a>> {
        let tag = fbb.create_string(&self.tag);
        let url = fbb.create_string(&self.url);

        schema::Tag::create(fbb, &schema::TagArgs {
            tag: Some(tag),
            url: Some(url),
            male: self.male,
            female: self.female,
        })
    }
}

// File
impl<'a> ToFlatBuffer<'a, schema::File<'a>> for gallery::File {
    fn to_flatbuffer(&self, fbb: &mut FlatBufferBuilder<'a>) -> WIPOffset<schema::File<'a>> {
        let name = fbb.create_string(&self.name);
        let hash = fbb.create_string(&self.hash);
        schema::File::create(fbb, &schema::FileArgs {
            name: Some(name),
            hash: Some(hash),
            width: self.width,
            height: self.height,
            hasavif: self.hasavif,
            haswebp: self.haswebp,
            hasjxl: self.hasjxl,
            single: self.single,
        })
    }
}
