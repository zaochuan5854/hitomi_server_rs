use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gallery_files")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub gallery_id: i32,

    #[sea_orm(primary_key)]
    pub file_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Gallery,
    File,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Gallery => Entity::belongs_to(super::gallery::Entity)
                .from(Column::GalleryId)
                .to(super::gallery::Column::Id)
                .into(),
            Self::File => Entity::belongs_to(super::file::Entity)
                .from(Column::FileId)
                .to(super::file::Column::Id)
                .into(),
        }
    }
}

impl Related<super::file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::File.def()
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
