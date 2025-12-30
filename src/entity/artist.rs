use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "artists")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub artist: String,
    pub url: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    GalleryArtist,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::GalleryArtist => Entity::has_many(super::gallery_artist::Entity).into(),
        }
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_artist::Relation::Gallery.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_artist::Relation::Artist.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
