use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "gallery_groups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub gallery_id: i32,

    #[sea_orm(primary_key)]
    pub group_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Gallery,
    Group,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Gallery => Entity::belongs_to(super::gallery::Entity)
                .from(Column::GalleryId)
                .to(super::gallery::Column::Id)
                .into(),
            Self::Group => Entity::belongs_to(super::group::Entity)
                .from(Column::GroupId)
                .to(super::group::Column::Id)
                .into(),
        }
    }
}

impl Related<super::group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl Related<super::gallery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Gallery.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
