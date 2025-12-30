use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gallery_parodies")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub gallery_id: i32,

    #[sea_orm(primary_key)]
    pub parody_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Gallery,
    Parody,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Gallery => Entity::belongs_to(super::gallery::Entity)
                .from(Column::GalleryId)
                .to(super::gallery::Column::Id)
                .into(),
            Self::Parody => Entity::belongs_to(super::parody::Entity)
                .from(Column::ParodyId)
                .to(super::parody::Column::Id)
                .into(),
        }
    }
}

impl Related<super::parody::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parody.def()
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
