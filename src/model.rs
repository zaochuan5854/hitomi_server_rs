use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{serde_as, DisplayFromStr, DeserializeAs, SerializeAs, DefaultOnNull, PickFirst, Same};

struct FlexibleBool;

impl<'de> DeserializeAs<'de, bool> for FlexibleBool {
    fn deserialize_as<D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        match v {
            serde_json::Value::Bool(b) => Ok(b),
            serde_json::Value::Number(n) => Ok(n.as_i64() == Some(1)),
            serde_json::Value::String(s) => Ok(s == "1" || s.to_lowercase() == "true"),
            _ => Ok(false),
        }
    }
}

impl SerializeAs<bool> for FlexibleBool {
    fn serialize_as<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 普通の true / false としてシリアライズ
        serializer.serialize_bool(*value)
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Gallery {
    // Required fields
    pub gallery_id: u32,
    pub title: String,
    pub date: String,
    pub files: Vec<File>,
    pub languages: Vec<Language>,
    pub scene_indexes: Vec<u32>,
    #[serde(rename = "type")]
    pub type_: String,
    
    // Required and dynamic type fields
    #[serde_as(as = "PickFirst<(DisplayFromStr, Same)>")]
    // [int | str] -> str
    pub id: String,
    #[serde_as(as = "Vec<PickFirst<(DisplayFromStr, Same)>>")]
    // [WrappedArray of int | WrappedArray of str] -> Vec<str>
    pub related: Vec<String>,

    // Required and nullable fields
    #[serde(default)]
    pub japanese_title: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub language_localname: Option<String>,
    #[serde(default)]
    pub language_url: Option<String>,
    #[serde(default)]
    pub video: Option<String>,
    #[serde(default)]
    pub videofilename: Option<String>,
    
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull")]
    pub artists: Vec<Artist>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull")]
    pub groups: Vec<Group>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull")]
    pub characters: Vec<Character>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull")]
    pub parodys: Vec<Parody>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull")]
    pub tags: Vec<Tag>,

    // Not required fields
    #[serde(default)]
    pub galleryurl: Option<String>,

    // Not required and nullable fields
    #[serde(default)]
    pub datepublished: Option<String>,

    // Not required, nullable and dynamic type fields
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull<FlexibleBool>")]
    pub blocked: bool,

}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Language {
    // Required fields
    pub name: String,
    pub language_localname: String,
    pub url: String,

    // Required and dynamic type fields
    #[serde_as(as = "PickFirst<(DisplayFromStr, Same)>")]
    // [int | str] -> str
    pub galleryid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artist {
    pub artist: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub group: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Character {
    pub character: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parody {
    pub parody: String,
    pub url: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    // Required fields
    pub tag: String,
    pub url: String,

    // Not required fields
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull<FlexibleBool>")]
    pub male: bool,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull<FlexibleBool>")]
    pub female: bool,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    // Required fields
    pub name: String,
    pub hash: String,
    pub width: u32,
    pub height: u32,

    // Required and dynamic type fields
    #[serde_as(as = "FlexibleBool")]
    pub hasavif: bool,

    // Not required fields
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull<FlexibleBool>")]
    pub haswebp: bool,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull<FlexibleBool>")]
    pub hasjxl: bool,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull<FlexibleBool>")]
    pub single: bool,
}
