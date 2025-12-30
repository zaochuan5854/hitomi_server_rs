use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "characters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub character: String,
    pub url: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    GalleryCharacter,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::GalleryCharacter => Entity::has_many(super::gallery_character::Entity).into(),
        }
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_character::Relation::Gallery.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_character::Relation::Character.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
