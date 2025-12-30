use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "files")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub name: String,
    pub hash: String,
    pub width: i32,
    pub height: i32,
    pub hasavif: bool,
    pub haswebp: bool,
    pub hasjxl: bool,
    pub single: bool,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    GalleryFile,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::GalleryFile => Entity::has_many(super::gallery_file::Entity).into(),
        }
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_file::Relation::Gallery.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_file::Relation::File.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
