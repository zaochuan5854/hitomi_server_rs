use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "parodies")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub parody: String,
    pub url: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    GalleryParody,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::GalleryParody => Entity::has_many(super::gallery_parody::Entity).into(),
        }
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_parody::Relation::Gallery.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_parody::Relation::Parody.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
