use sea_orm::*;
use crate::domain;
use crate::entity::{self, prelude::*};

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


