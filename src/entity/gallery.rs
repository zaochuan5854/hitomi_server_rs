use sea_orm::entity::prelude::*;
use chrono::{DateTime, FixedOffset, NaiveDate};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "galleries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub gallery_id: i32,
    pub title: String,
    pub date: DateTime<FixedOffset>,
    #[sea_orm(column_name = "type")]
    pub type_: String,
    pub external_id: String,
    pub scene_indexes: Vec<i32>,
    pub related_ids: Vec<String>,
    pub japanese_title: Option<String>,
    
    pub language_id: Option<i32>,
    pub translation_group_id: Vec<String>,

    pub video: Option<String>,
    pub videofilename: Option<String>,
    pub gallery_url: Option<String>,
    pub date_published: Option<NaiveDate>,
    pub blocked: bool,
    pub files: Json,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Language,
    GalleryTag,
    GalleryArtist,
    GalleryGroup,
    GalleryCharacter,
    GalleryParody,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Language => Entity::belongs_to(super::language::Entity)
                .from(Column::LanguageId)
                .to(super::language::Column::Id)
                .into(),
            Self::GalleryTag => Entity::has_many(super::gallery_tag::Entity).into(),
            Self::GalleryArtist => Entity::has_many(super::gallery_artist::Entity).into(),
            Self::GalleryGroup => Entity::has_many(super::gallery_group::Entity).into(),
            Self::GalleryCharacter => Entity::has_many(super::gallery_character::Entity).into(),
            Self::GalleryParody => Entity::has_many(super::gallery_parody::Entity).into(),
        }
    }
}

impl Related<super::language::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Language.def()
    }
}

impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_tag::Relation::Tag.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_tag::Relation::Gallery.def().rev())
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_artist::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_artist::Relation::Gallery.def().rev())
    }
}

impl Related<super::group::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_group::Relation::Group.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_group::Relation::Gallery.def().rev())
    }
}

impl Related<super::character::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_character::Relation::Character.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_character::Relation::Gallery.def().rev())
    }
}

impl Related<super::parody::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_parody::Relation::Parody.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_parody::Relation::Gallery.def().rev())
    }
}



impl ActiveModelBehavior for ActiveModel {}
