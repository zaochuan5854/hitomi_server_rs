pub mod gallery;
pub mod language;
pub mod artist;
pub mod group;
pub mod character;
pub mod parody;
pub mod tag;
pub mod gallery_artist;
pub mod gallery_group;
pub mod gallery_character;
pub mod gallery_parody;
pub mod gallery_tag;


pub mod prelude {
    pub use super::gallery::Entity as Gallery;
    pub use super::language::Entity as Language;
    pub use super::artist::Entity as Artist;
    pub use super::group::Entity as Group;
    pub use super::character::Entity as Character;
    pub use super::parody::Entity as Parody;
    pub use super::tag::Entity as Tag;
}