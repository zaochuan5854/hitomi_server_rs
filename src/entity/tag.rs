use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "tags")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(column_name = "name")]
    pub name: String,

    pub url: String,

    pub male: bool,
    pub female: bool,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    GalleryTag,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::GalleryTag => Entity::has_many(super::gallery_tag::Entity).into(),
        }
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_tag::Relation::Gallery.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_tag::Relation::Tag.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
