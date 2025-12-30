use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "groups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub group: String,
    pub url: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    GalleryGroup,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::GalleryGroup => Entity::has_many(super::gallery_group::Entity).into(),
        }
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        super::gallery_group::Relation::Gallery.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::gallery_group::Relation::Group.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
