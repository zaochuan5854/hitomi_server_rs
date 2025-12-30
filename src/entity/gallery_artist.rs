use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gallery_artists")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub gallery_id: i32,

    #[sea_orm(primary_key)]
    pub artist_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Gallery,
    Artist,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Gallery => Entity::belongs_to(super::gallery::Entity)
                .from(Column::GalleryId)
                .to(super::gallery::Column::Id)
                .into(),
            Self::Artist => Entity::belongs_to(super::artist::Entity)
                .from(Column::ArtistId)
                .to(super::artist::Column::Id)
                .into(),
        }
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Artist.def()
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
