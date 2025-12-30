use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{serde_as, DeserializeAs, SerializeAs, DefaultOnNull};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Gallery {
    // Required fields
    pub gallery_id: i32,
    pub title: String,
    pub date: String,
    pub files: Vec<File>,
    pub languages: Vec<Language>,
    pub scene_indexes: Vec<i32>,
    #[serde(rename = "type")]
    pub type_: String,
    
    // Required and dynamic type fields
    #[serde_as(as = "FlexibleString")]
    // [int | str] -> str
    pub id: String,
    #[serde_as(as = "Vec<FlexibleString>")]
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
    #[serde(rename = "parodies", alias = "parodys")]
    pub parodies: Vec<Parody>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnNull")]
    pub tags: Vec<Tag>,

    // Not required fields
    #[serde(default)]
    #[serde(rename = "gallery_url", alias = "galleryurl")]
    pub gallery_url: Option<String>,

    // Not required and nullable fields
    #[serde(default)]
    #[serde(rename = "date_published", alias = "datepublished")]
    pub date_published: Option<String>,

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
    #[serde_as(as = "FlexibleString")]
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
    pub width: i32,
    pub height: i32,

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

struct FlexibleString;

impl<'de> DeserializeAs<'de, String> for FlexibleString {
    fn deserialize_as<D>(deserializer: D) -> Result<String, D::Error>
    where D: Deserializer<'de> {
        // Untaggedな列挙型をローカルで定義して、Serdeに型判定を任せる
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrInt {
            Str(String),
            Int(i64),
            Float(f64),
        }

        match StringOrInt::deserialize(deserializer)? {
            StringOrInt::Str(s) => Ok(s),
            StringOrInt::Int(i) => Ok(i.to_string()),
            StringOrInt::Float(f) => Ok(f.to_string()),
        }
    }
}

impl SerializeAs<String>  for FlexibleString {
    fn serialize_as<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        // そのまま文字列としてシリアライズ
        serializer.serialize_str(value)
    }
    
}

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
