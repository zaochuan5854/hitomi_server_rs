use sea_orm::*;
use crate::domain;
use crate::entity::{self, prelude::*};
use sea_orm::sea_query::OnConflict;

/// 複数の Gallery を一括で永続化する
pub async fn insert_many_galleries(
    db: &DatabaseConnection,
    galleries: Vec<domain::gallery::Gallery>,
) -> Result<(), DbErr> {
    if galleries.is_empty() {
        return Ok(());
    }

    let txn = db.begin().await
        .map_err(|e| DbErr::Custom(format!("Failed to begin transaction: {}", e)))?;

    // 1. 関連エンティティの一括 Upsert と ID マッピングの作成
    // 各エンティティごとにユニークなリストを作成して Upsert し、
    // (name/key -> id) の Map を作る

    // Language
    let languages: std::collections::HashMap<String, i32> = upsert_languages(&txn, &galleries).await
        .map_err(|e| DbErr::Custom(format!("Failed to upsert languages: {}", e)))?;

    // Tag
    let tags: std::collections::HashMap<(String, bool, bool), i32> = upsert_tags(&txn, &galleries).await
        .map_err(|e| DbErr::Custom(format!("Failed to upsert tags: {}", e)))?;

    // Artist
    let artists: std::collections::HashMap<String, i32> = upsert_artists(&txn, &galleries).await
        .map_err(|e| DbErr::Custom(format!("Failed to upsert artists: {}", e)))?;

    // Group
    let groups: std::collections::HashMap<String, i32> = upsert_groups(&txn, &galleries).await
        .map_err(|e| DbErr::Custom(format!("Failed to upsert groups: {}", e)))?;

    // Character
    let characters: std::collections::HashMap<String, i32> = upsert_characters(&txn, &galleries).await
        .map_err(|e| DbErr::Custom(format!("Failed to upsert characters: {}", e)))?;

    // Parody
    let parodies: std::collections::HashMap<String, i32> = upsert_parodies(&txn, &galleries).await
        .map_err(|e| DbErr::Custom(format!("Failed to upsert parodies: {}", e)))?;

    // 2. Gallery の一括 Upsert
    // 戻り値として (gallery_id -> id) の Map を取得したいが、
    // insert_many は ID を返さない場合があるため、
    // ここでは「gallery_id」をキーにして Upsert し、その後必要な ID を取得する戦略をとるか、
    // あるいは単純に Upsert する。
    // 中間テーブルへの挿入には `galleries` テーブルのプライマリキー `id` が必要。
    // `gallery_id` (ユニークキー) から `id` を引けるようにする。

    let gallery_models: Vec<entity::gallery::ActiveModel> = galleries.iter().map(|g| {
        let lang_id = g.language.as_ref().and_then(|l| languages.get(l)).cloned();
        let translation_group_id: Vec<String> = g.languages.iter()
            .map(|lang| lang.galleryid.clone())
            .collect();

        entity::gallery::ActiveModel {
            gallery_id: Set(g.gallery_id),
            title: Set(g.title.clone()),
            date: Set(g.date.clone()),
            type_: Set(g.type_.clone()),
            external_id: Set(g.id.clone()),
            scene_indexes: Set(g.scene_indexes.clone()),
            related_ids: Set(g.related.clone()),
            japanese_title: Set(g.japanese_title.clone()),
            language_id: Set(lang_id),
            translation_group_id: Set(translation_group_id),
            video: Set(g.video.clone()),
            videofilename: Set(g.videofilename.clone()),
            gallery_url: Set(g.gallery_url.clone()),
            date_published: Set(g.date_published.clone()),
            blocked: Set(g.blocked),
            files: Set(serde_json::to_value(&g.files).unwrap()),
            ..Default::default()
        }
    }).collect();

    // Gallery Upsert
    // ON CONFLICT (gallery_id) DO UPDATE ...
    // exec_with_returning は ON CONFLICT UPDATE の場合、期待通りに動作しないことがある
    // 代わりに exec を使い、その後に SELECT で ID を取得する
    
    let on_conflict = OnConflict::column(entity::gallery::Column::GalleryId)
        .update_columns([
            entity::gallery::Column::Title,
            entity::gallery::Column::Date,
            entity::gallery::Column::Type,
            entity::gallery::Column::ExternalId,
            entity::gallery::Column::SceneIndexes,
            entity::gallery::Column::RelatedIds,
            entity::gallery::Column::JapaneseTitle,
            entity::gallery::Column::LanguageId,
            entity::gallery::Column::TranslationGroupId,
            entity::gallery::Column::Video,
            entity::gallery::Column::Videofilename,
            entity::gallery::Column::GalleryUrl,
            entity::gallery::Column::DatePublished,
            entity::gallery::Column::Blocked,
            entity::gallery::Column::Files,
        ])
        .to_owned();

    // insert_many を実行（ON CONFLICT 付き）
    Gallery::insert_many(gallery_models)
        .on_conflict(on_conflict)
        .exec(&txn)
        .await?;
    
    // 挿入した gallery_id のリスト
    let target_gallery_ids: Vec<i32> = galleries.iter().map(|g| g.gallery_id).collect();
    
    // ID マップ作成 (gallery_id -> id)
    // チャンクサイズが100程度なら IN 句で引いても問題ない
    let gallery_map: std::collections::HashMap<i32, i32> = Gallery::find()
        .filter(entity::gallery::Column::GalleryId.is_in(target_gallery_ids.clone()))
        .all(&txn)
        .await?
        .into_iter()
        .map(|m| (m.gallery_id, m.id))
        .collect();

    // 挿入されたレコード数をチェック
    if gallery_map.is_empty() {
        return Err(DbErr::Custom(format!(
            "Gallery map is empty after insert. Expected {} galleries. Target IDs: {:?}",
            galleries.len(),
            target_gallery_ids.iter().take(5).collect::<Vec<_>>()
        )));
    }
    
    if gallery_map.len() != galleries.len() {
        return Err(DbErr::Custom(format!(
            "Expected {} galleries but found {} in database. Missing gallery_ids: {:?}",
            galleries.len(),
            gallery_map.len(),
            target_gallery_ids.iter()
                .filter(|gid| !gallery_map.contains_key(gid))
                .take(10)
                .collect::<Vec<_>>()
        )));
    }

    // 3. 中間テーブルの一括 Insert
    // 中間テーブルは (gallery_id, other_id) のペア。
    // 重複エラーを避けるため、ON CONFLICT DO NOTHING を使う。

    let mut gallery_tags = Vec::new();
    let mut gallery_artists = Vec::new();
    let mut gallery_groups = Vec::new();
    let mut gallery_characters = Vec::new();
    let mut gallery_parodies = Vec::new();

    for gallery in &galleries {
        if let Some(&gid) = gallery_map.get(&gallery.gallery_id) {
            // Tags
            for tag in &gallery.tags {
                if let Some(&tid) = tags.get(&(tag.tag.clone(), tag.male, tag.female)) {
                    gallery_tags.push(entity::gallery_tag::ActiveModel {
                        gallery_id: Set(gid),
                        tag_id: Set(tid),
                    });
                }
            }
            // Artists
            for artist in &gallery.artists {
                if let Some(&aid) = artists.get(&artist.artist) {
                    gallery_artists.push(entity::gallery_artist::ActiveModel {
                        gallery_id: Set(gid),
                        artist_id: Set(aid),
                    });
                }
            }
            // Groups
            for group in &gallery.groups {
                if let Some(&grid) = groups.get(&group.group) {
                    gallery_groups.push(entity::gallery_group::ActiveModel {
                        gallery_id: Set(gid),
                        group_id: Set(grid),
                    });
                }
            }
            // Characters
            for character in &gallery.characters {
                if let Some(&cid) = characters.get(&character.character) {
                    gallery_characters.push(entity::gallery_character::ActiveModel {
                        gallery_id: Set(gid),
                        character_id: Set(cid),
                    });
                }
            }
            // Parodies
            for parody in &gallery.parodies {
                if let Some(&pid) = parodies.get(&parody.parody) {
                    gallery_parodies.push(entity::gallery_parody::ActiveModel {
                        gallery_id: Set(gid),
                        parody_id: Set(pid),
                    });
                }
            }
        }
    }

    // 一括挿入実行 (ON CONFLICT DO NOTHING)
    if !gallery_tags.is_empty() {
        entity::gallery_tag::Entity::insert_many(gallery_tags)
            .on_conflict(OnConflict::columns([entity::gallery_tag::Column::GalleryId, entity::gallery_tag::Column::TagId]).do_nothing().to_owned())
            .exec(&txn).await?;
    }
    if !gallery_artists.is_empty() {
        entity::gallery_artist::Entity::insert_many(gallery_artists)
            .on_conflict(OnConflict::columns([entity::gallery_artist::Column::GalleryId, entity::gallery_artist::Column::ArtistId]).do_nothing().to_owned())
            .exec(&txn).await?;
    }
    if !gallery_groups.is_empty() {
        entity::gallery_group::Entity::insert_many(gallery_groups)
            .on_conflict(OnConflict::columns([entity::gallery_group::Column::GalleryId, entity::gallery_group::Column::GroupId]).do_nothing().to_owned())
            .exec(&txn).await?;
    }
    if !gallery_characters.is_empty() {
        entity::gallery_character::Entity::insert_many(gallery_characters)
            .on_conflict(OnConflict::columns([entity::gallery_character::Column::GalleryId, entity::gallery_character::Column::CharacterId]).do_nothing().to_owned())
            .exec(&txn).await?;
    }
    if !gallery_parodies.is_empty() {
        entity::gallery_parody::Entity::insert_many(gallery_parodies)
            .on_conflict(OnConflict::columns([entity::gallery_parody::Column::GalleryId, entity::gallery_parody::Column::ParodyId]).do_nothing().to_owned())
            .exec(&txn).await?;
    }

    txn.commit().await?;
    Ok(())
}

// ========================================
// Private Helper Functions (Batch)
// ========================================

async fn upsert_languages(
    db: &DatabaseTransaction,
    galleries: &[domain::gallery::Gallery],
) -> Result<std::collections::HashMap<String, i32>, DbErr> {
    let mut map = std::collections::HashMap::new();
    let mut to_insert = std::collections::HashMap::new();

    for g in galleries {
        if let Some(name) = &g.language {
            if !map.contains_key(name) {
                to_insert.insert(name.clone(), (g.language_localname.clone(), g.language_url.clone()));
            }
        }
    }

    if to_insert.is_empty() {
        return Ok(map);
    }

    // 既存チェック
    let names: Vec<String> = to_insert.keys().cloned().collect();
    let existing = Language::find()
        .filter(entity::language::Column::Name.is_in(names.clone()))
        .all(db)
        .await?;

    for m in existing {
        map.insert(m.name.clone(), m.id);
        to_insert.remove(&m.name);
    }

    // 新規挿入
    if !to_insert.is_empty() {
        let models: Vec<entity::language::ActiveModel> = to_insert.into_iter().map(|(name, (local, url))| {
            entity::language::ActiveModel {
                name: Set(name),
                local_name: Set(local),
                url: Set(url),
                ..Default::default()
            }
        }).collect();

        // ON CONFLICT DO UPDATE（デッドロック回避のため）
        Language::insert_many(models)
            .on_conflict(
                OnConflict::column(entity::language::Column::Name)
                    .update_columns([entity::language::Column::LocalName, entity::language::Column::Url])
                    .to_owned()
            )
            .exec(db)
            .await?;
        
        // 全取得してマップを完成させる
        let existing_after = Language::find()
            .filter(entity::language::Column::Name.is_in(names))
            .all(db)
            .await?;
        for m in existing_after {
            map.insert(m.name, m.id);
        }
    }

    Ok(map)
}

async fn upsert_tags(
    db: &DatabaseTransaction,
    galleries: &[domain::gallery::Gallery],
) -> Result<std::collections::HashMap<(String, bool, bool), i32>, DbErr> {
    let mut map = std::collections::HashMap::new();
    let mut to_insert = std::collections::HashMap::new();

    for g in galleries {
        for t in &g.tags {
            let key = (t.tag.clone(), t.male, t.female);
            if !map.contains_key(&key) {
                to_insert.insert(key, t.url.clone());
            }
        }
    }

    if to_insert.is_empty() {
        return Ok(map);
    }

    // 既存チェックは複雑になるので、ON CONFLICT DO NOTHING で一括挿入してから全取得する戦略
    let models: Vec<entity::tag::ActiveModel> = to_insert.iter().map(|((name, male, female), url)| {
        entity::tag::ActiveModel {
            name: Set(name.clone()),
            url: Set(url.clone()),
            male: Set(*male),
            female: Set(*female),
            ..Default::default()
        }
    }).collect();

    if !models.is_empty() {
        Tag::insert_many(models)
            .on_conflict(
                OnConflict::columns([entity::tag::Column::Name, entity::tag::Column::Male, entity::tag::Column::Female])
                .update_column(entity::tag::Column::Url)
                .to_owned()
            )
            .exec(db)
            .await?;
    }

    // ID取得
    // 検索条件を作るのが面倒なので、名前リストで引いてからメモリ上でフィルタする
    // (タグ名は重複しうるが、(name, male, female) はユニーク)
    let names: Vec<String> = to_insert.keys().map(|(n, _, _)| n.clone()).collect();
    // チャンク内のタグ種類数が多いと IN 句が大きくなりすぎる可能性はあるが、100件程度なら許容範囲
    
    let stored_tags = Tag::find()
        .filter(entity::tag::Column::Name.is_in(names))
        .all(db)
        .await?;

    for t in stored_tags {
        map.insert((t.name, t.male, t.female), t.id);
    }

    Ok(map)
}

async fn upsert_artists(
    db: &DatabaseTransaction,
    galleries: &[domain::gallery::Gallery],
) -> Result<std::collections::HashMap<String, i32>, DbErr> {
    let mut map = std::collections::HashMap::new();
    let mut to_insert = std::collections::HashMap::new();

    for g in galleries {
        for a in &g.artists {
            if !map.contains_key(&a.artist) {
                to_insert.insert(a.artist.clone(), a.url.clone());
            }
        }
    }

    if to_insert.is_empty() {
        return Ok(map);
    }

    let models: Vec<entity::artist::ActiveModel> = to_insert.iter().map(|(name, url)| {
        entity::artist::ActiveModel {
            artist: Set(name.clone()),
            url: Set(url.clone()),
            ..Default::default()
        }
    }).collect();

    if !models.is_empty() {
        Artist::insert_many(models)
            .on_conflict(
                OnConflict::column(entity::artist::Column::Artist)
                    .update_column(entity::artist::Column::Url)
                    .to_owned()
            )
            .exec(db)
            .await?;
    }

    let names: Vec<String> = to_insert.keys().cloned().collect();
    let stored = Artist::find()
        .filter(entity::artist::Column::Artist.is_in(names))
        .all(db)
        .await?;

    for m in stored {
        map.insert(m.artist, m.id);
    }

    Ok(map)
}

async fn upsert_groups(
    db: &DatabaseTransaction,
    galleries: &[domain::gallery::Gallery],
) -> Result<std::collections::HashMap<String, i32>, DbErr> {
    let mut map = std::collections::HashMap::new();
    let mut to_insert = std::collections::HashMap::new();

    for g in galleries {
        for item in &g.groups {
            if !map.contains_key(&item.group) {
                to_insert.insert(item.group.clone(), item.url.clone());
            }
        }
    }

    if to_insert.is_empty() {
        return Ok(map);
    }

    let models: Vec<entity::group::ActiveModel> = to_insert.iter().map(|(name, url)| {
        entity::group::ActiveModel {
            group: Set(name.clone()),
            url: Set(url.clone()),
            ..Default::default()
        }
    }).collect();

    if !models.is_empty() {
        Group::insert_many(models)
            .on_conflict(
                OnConflict::column(entity::group::Column::Group)
                    .update_column(entity::group::Column::Url)
                    .to_owned()
            )
            .exec(db)
            .await?;
    }

    let names: Vec<String> = to_insert.keys().cloned().collect();
    let stored = Group::find()
        .filter(entity::group::Column::Group.is_in(names))
        .all(db)
        .await?;

    for m in stored {
        map.insert(m.group, m.id);
    }

    Ok(map)
}

async fn upsert_characters(
    db: &DatabaseTransaction,
    galleries: &[domain::gallery::Gallery],
) -> Result<std::collections::HashMap<String, i32>, DbErr> {
    let mut map = std::collections::HashMap::new();
    let mut to_insert = std::collections::HashMap::new();

    for g in galleries {
        for item in &g.characters {
            if !map.contains_key(&item.character) {
                to_insert.insert(item.character.clone(), item.url.clone());
            }
        }
    }

    if to_insert.is_empty() {
        return Ok(map);
    }

    let models: Vec<entity::character::ActiveModel> = to_insert.iter().map(|(name, url)| {
        entity::character::ActiveModel {
            character: Set(name.clone()),
            url: Set(url.clone()),
            ..Default::default()
        }
    }).collect();

    if !models.is_empty() {
        Character::insert_many(models)
            .on_conflict(
                OnConflict::column(entity::character::Column::Character)
                    .update_column(entity::character::Column::Url)
                    .to_owned()
            )
            .exec(db)
            .await?;
    }

    let names: Vec<String> = to_insert.keys().cloned().collect();
    let stored = Character::find()
        .filter(entity::character::Column::Character.is_in(names))
        .all(db)
        .await?;

    for m in stored {
        map.insert(m.character, m.id);
    }

    Ok(map)
}

async fn upsert_parodies(
    db: &DatabaseTransaction,
    galleries: &[domain::gallery::Gallery],
) -> Result<std::collections::HashMap<String, i32>, DbErr> {
    let mut map = std::collections::HashMap::new();
    let mut to_insert = std::collections::HashMap::new();

    for g in galleries {
        for item in &g.parodies {
            if !map.contains_key(&item.parody) {
                to_insert.insert(item.parody.clone(), item.url.clone());
            }
        }
    }

    if to_insert.is_empty() {
        return Ok(map);
    }

    let models: Vec<entity::parody::ActiveModel> = to_insert.iter().map(|(name, url)| {
        entity::parody::ActiveModel {
            parody: Set(name.clone()),
            url: Set(url.clone()),
            ..Default::default()
        }
    }).collect();

    if !models.is_empty() {
        Parody::insert_many(models)
            .on_conflict(
                OnConflict::column(entity::parody::Column::Parody)
                    .update_column(entity::parody::Column::Url)
                    .to_owned()
            )
            .exec(db)
            .await?;
    }

    let names: Vec<String> = to_insert.keys().cloned().collect();
    let stored = Parody::find()
        .filter(entity::parody::Column::Parody.is_in(names))
        .all(db)
        .await?;

    for m in stored {
        map.insert(m.parody, m.id);
    }

    Ok(map)
}

/// Gallery を永続化する（トランザクション内で完結）
pub async fn insert_gallery(
    db: &DatabaseConnection,
    gallery: domain::gallery::Gallery,
) -> Result<(), DbErr> {
    let txn = db.begin().await?;

    // 1. Language の upsert（name で検索/作成）
    let language_id = if let Some(lang_name) = gallery.language.as_ref() {
        Some(
            upsert_language(
                &txn,
                lang_name,
                gallery.language_localname.as_deref(),
                gallery.language_url.as_deref(),
            )
            .await?,
        )
    } else {
        None
    };

    // 2. Gallery の upsert（gallery_id をユニークキーとして使用）
    let gallery_id = upsert_gallery(&txn, &gallery, language_id).await?;

    // 3. Tag の upsert と link
    for tag in &gallery.tags {
        let tag_id = upsert_tag(&txn, &tag.tag, &tag.url, tag.male, tag.female).await?;
        link_gallery_tag(&txn, gallery_id, tag_id).await?;
    }

    // 4. Artist の upsert と link
    for artist in &gallery.artists {
        let artist_id = upsert_artist(&txn, &artist.artist, &artist.url).await?;
        link_gallery_artist(&txn, gallery_id, artist_id).await?;
    }

    // 5. Group の upsert と link
    for group in &gallery.groups {
        let group_id = upsert_group(&txn, &group.group, &group.url).await?;
        link_gallery_group(&txn, gallery_id, group_id).await?;
    }

    // 6. Character の upsert と link
    for character in &gallery.characters {
        let character_id = upsert_character(&txn, &character.character, &character.url).await?;
        link_gallery_character(&txn, gallery_id, character_id).await?;
    }

    // 7. Parody の upsert と link
    for parody in &gallery.parodies {
        let parody_id = upsert_parody(&txn, &parody.parody, &parody.url).await?;
        link_gallery_parody(&txn, gallery_id, parody_id).await?;
    }

    txn.commit().await?;
    Ok(())
}

// ========================================
// Private Helper Functions
// ========================================

/// Gallery を upsert（gallery_id で判定）
async fn upsert_gallery(
    db: &DatabaseTransaction,
    gallery: &domain::gallery::Gallery,
    language_id: Option<i32>,
) -> Result<i32, DbErr> {
    // gallery_id で既存レコードを検索
    let existing = Gallery::find()
        .filter(entity::gallery::Column::GalleryId.eq(gallery.gallery_id))
        .one(db)
        .await?;

    // translation_group_id を languages から取得（全ての galleryid を収集）
    let translation_group_id: Vec<String> = gallery.languages.iter()
        .map(|lang| lang.galleryid.clone())
        .collect();

    let gallery_model = entity::gallery::ActiveModel {
        id: existing.as_ref().map(|g| Set(g.id)).unwrap_or(NotSet),
        gallery_id: Set(gallery.gallery_id),
        title: Set(gallery.title.clone()),
        date: Set(gallery.date.clone()),
        type_: Set(gallery.type_.clone()),
        external_id: Set(gallery.id.clone()),
        scene_indexes: Set(gallery.scene_indexes.clone()),
        related_ids: Set(gallery.related.clone()),
        japanese_title: Set(gallery.japanese_title.clone()),
        language_id: Set(language_id),
        translation_group_id: Set(translation_group_id),
        video: Set(gallery.video.clone()),
        videofilename: Set(gallery.videofilename.clone()),
        gallery_url: Set(gallery.gallery_url.clone()),
        date_published: Set(gallery.date_published.clone()),
        blocked: Set(gallery.blocked),
        files: Set(serde_json::to_value(&gallery.files).unwrap()),
    };

    let result = if existing.is_some() {
        gallery_model.update(db).await?
    } else {
        gallery_model.insert(db).await?
    };

    Ok(result.id)
}

/// Language を upsert（name で判定）
async fn upsert_language(
    db: &DatabaseTransaction,
    name: &str,
    local_name: Option<&str>,
    url: Option<&str>,
) -> Result<i32, DbErr> {
    // name で既存レコードを検索
    let existing = Language::find()
        .filter(entity::language::Column::Name.eq(name))
        .one(db)
        .await?;

    let language_model = entity::language::ActiveModel {
        id: existing.as_ref().map(|l| Set(l.id)).unwrap_or(NotSet),
        name: Set(name.to_string()),
        // null で上書きしない（既存値がある場合は新しい値が Some の時のみ更新）
        local_name: Set(match (local_name, existing.as_ref()) {
            (Some(new_val), _) => Some(new_val.to_string()),
            (None, Some(ex)) => ex.local_name.clone(),
            (None, None) => None,
        }),
        url: Set(match (url, existing.as_ref()) {
            (Some(new_val), _) => Some(new_val.to_string()),
            (None, Some(ex)) => ex.url.clone(),
            (None, None) => None,
        }),
    };

    let result = if existing.is_some() {
        language_model.update(db).await?
    } else {
        language_model.insert(db).await?
    };

    Ok(result.id)
}

/// Tag を upsert（name + male + female で判定）
async fn upsert_tag(
    db: &DatabaseTransaction,
    name: &str,
    url: &str,
    male: bool,
    female: bool,
) -> Result<i32, DbErr> {
    let existing = Tag::find()
        .filter(entity::tag::Column::Name.eq(name))
        .filter(entity::tag::Column::Male.eq(male))
        .filter(entity::tag::Column::Female.eq(female))
        .one(db)
        .await?;

    let tag_model = entity::tag::ActiveModel {
        id: existing.as_ref().map(|t| Set(t.id)).unwrap_or(NotSet),
        name: Set(name.to_string()),
        url: Set(url.to_string()),
        male: Set(male),
        female: Set(female),
    };

    let result = if existing.is_some() {
        tag_model.update(db).await?
    } else {
        tag_model.insert(db).await?
    };

    Ok(result.id)
}

/// Gallery–Tag を link（重複は無視）
async fn link_gallery_tag(
    db: &DatabaseTransaction,
    gallery_id: i32,
    tag_id: i32,
) -> Result<(), DbErr> {
    // 既存チェック
    let existing = entity::gallery_tag::Entity::find()
        .filter(entity::gallery_tag::Column::GalleryId.eq(gallery_id))
        .filter(entity::gallery_tag::Column::TagId.eq(tag_id))
        .one(db)
        .await?;

    if existing.is_none() {
        let link = entity::gallery_tag::ActiveModel {
            gallery_id: Set(gallery_id),
            tag_id: Set(tag_id),
        };
        link.insert(db).await?;
    }

    Ok(())
}

/// Artist を upsert（artist 名で判定）
async fn upsert_artist(
    db: &DatabaseTransaction,
    artist_name: &str,
    url: &str,
) -> Result<i32, DbErr> {
    let existing = Artist::find()
        .filter(entity::artist::Column::Artist.eq(artist_name))
        .one(db)
        .await?;

    let artist_model = entity::artist::ActiveModel {
        id: existing.as_ref().map(|a| Set(a.id)).unwrap_or(NotSet),
        artist: Set(artist_name.to_string()),
        url: Set(url.to_string()),
    };

    let result = if existing.is_some() {
        artist_model.update(db).await?
    } else {
        artist_model.insert(db).await?
    };

    Ok(result.id)
}

async fn link_gallery_artist(
    db: &DatabaseTransaction,
    gallery_id: i32,
    artist_id: i32,
) -> Result<(), DbErr> {
    let existing = entity::gallery_artist::Entity::find()
        .filter(entity::gallery_artist::Column::GalleryId.eq(gallery_id))
        .filter(entity::gallery_artist::Column::ArtistId.eq(artist_id))
        .one(db)
        .await?;

    if existing.is_none() {
        let link = entity::gallery_artist::ActiveModel {
            gallery_id: Set(gallery_id),
            artist_id: Set(artist_id),
        };
        link.insert(db).await?;
    }

    Ok(())
}

/// Group を upsert（group 名で判定）
async fn upsert_group(
    db: &DatabaseTransaction,
    group_name: &str,
    url: &str,
) -> Result<i32, DbErr> {
    let existing = Group::find()
        .filter(entity::group::Column::Group.eq(group_name))
        .one(db)
        .await?;

    let group_model = entity::group::ActiveModel {
        id: existing.as_ref().map(|g| Set(g.id)).unwrap_or(NotSet),
        group: Set(group_name.to_string()),
        url: Set(url.to_string()),
    };

    let result = if existing.is_some() {
        group_model.update(db).await?
    } else {
        group_model.insert(db).await?
    };

    Ok(result.id)
}

async fn link_gallery_group(
    db: &DatabaseTransaction,
    gallery_id: i32,
    group_id: i32,
) -> Result<(), DbErr> {
    let existing = entity::gallery_group::Entity::find()
        .filter(entity::gallery_group::Column::GalleryId.eq(gallery_id))
        .filter(entity::gallery_group::Column::GroupId.eq(group_id))
        .one(db)
        .await?;

    if existing.is_none() {
        let link = entity::gallery_group::ActiveModel {
            gallery_id: Set(gallery_id),
            group_id: Set(group_id),
        };
        link.insert(db).await?;
    }

    Ok(())
}

/// Character を upsert（character 名で判定）
async fn upsert_character(
    db: &DatabaseTransaction,
    character_name: &str,
    url: &str,
) -> Result<i32, DbErr> {
    let existing = Character::find()
        .filter(entity::character::Column::Character.eq(character_name))
        .one(db)
        .await?;

    let character_model = entity::character::ActiveModel {
        id: existing.as_ref().map(|c| Set(c.id)).unwrap_or(NotSet),
        character: Set(character_name.to_string()),
        url: Set(url.to_string()),
    };

    let result = if existing.is_some() {
        character_model.update(db).await?
    } else {
        character_model.insert(db).await?
    };

    Ok(result.id)
}

async fn link_gallery_character(
    db: &DatabaseTransaction,
    gallery_id: i32,
    character_id: i32,
) -> Result<(), DbErr> {
    let existing = entity::gallery_character::Entity::find()
        .filter(entity::gallery_character::Column::GalleryId.eq(gallery_id))
        .filter(entity::gallery_character::Column::CharacterId.eq(character_id))
        .one(db)
        .await?;

    if existing.is_none() {
        let link = entity::gallery_character::ActiveModel {
            gallery_id: Set(gallery_id),
            character_id: Set(character_id),
        };
        link.insert(db).await?;
    }

    Ok(())
}

/// Parody を upsert（parody 名で判定）
async fn upsert_parody(
    db: &DatabaseTransaction,
    parody_name: &str,
    url: &str,
) -> Result<i32, DbErr> {
    let existing = Parody::find()
        .filter(entity::parody::Column::Parody.eq(parody_name))
        .one(db)
        .await?;

    let parody_model = entity::parody::ActiveModel {
        id: existing.as_ref().map(|p| Set(p.id)).unwrap_or(NotSet),
        parody: Set(parody_name.to_string()),
        url: Set(url.to_string()),
    };

    let result = if existing.is_some() {
        parody_model.update(db).await?
    } else {
        parody_model.insert(db).await?
    };

    Ok(result.id)
}

async fn link_gallery_parody(
    db: &DatabaseTransaction,
    gallery_id: i32,
    parody_id: i32,
) -> Result<(), DbErr> {
    let existing = entity::gallery_parody::Entity::find()
        .filter(entity::gallery_parody::Column::GalleryId.eq(gallery_id))
        .filter(entity::gallery_parody::Column::ParodyId.eq(parody_id))
        .one(db)
        .await?;

    if existing.is_none() {
        let link = entity::gallery_parody::ActiveModel {
            gallery_id: Set(gallery_id),
            parody_id: Set(parody_id),
        };
        link.insert(db).await?;
    }

    Ok(())
}


