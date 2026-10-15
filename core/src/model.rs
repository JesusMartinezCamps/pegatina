use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Text,
    Link,
    Image,
    Files,
    Secret,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Link => "link",
            Kind::Image => "image",
            Kind::Files => "files",
            Kind::Secret => "secret",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "text" => Some(Kind::Text),
            "link" => Some(Kind::Link),
            "image" => Some(Kind::Image),
            "files" => Some(Kind::Files),
            "secret" => Some(Kind::Secret),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: i64,
    pub kind: Kind,
    pub preview: String,
    pub text: Option<String>,
    pub image_file: Option<String>,
    pub files: Vec<String>,
    pub source_app: Option<String>,
    pub favorite: bool,
    pub space_id: Option<i64>,
    pub created_at: i64,
    pub used_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Space {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub autostart: bool,
    pub hotkey: String,
    pub text_dir: Option<String>,
    pub media_dir: Option<String>,
    pub history_limit: u32,
    pub save_passwords: bool,
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            autostart: true,
            hotkey: "Ctrl+Alt+Space".into(),
            text_dir: None,
            media_dir: None,
            history_limit: 1000,
            save_passwords: false,
            theme: "system".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListFilter {
    pub view: String,
    pub space_id: Option<i64>,
    pub kind: Option<Kind>,
    pub query: Option<String>,
    pub limit: Option<u32>,
}

impl Default for ListFilter {
    fn default() -> Self {
        ListFilter {
            view: "recent".into(),
            space_id: None,
            kind: None,
            query: None,
            limit: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_serializes_in_camel_case_with_lowercase_kind() {
        let item = Item {
            id: 1,
            kind: Kind::Link,
            preview: "p".into(),
            text: Some("t".into()),
            image_file: None,
            files: vec!["a".into()],
            source_app: Some("Brave".into()),
            favorite: true,
            space_id: Some(2),
            created_at: 10,
            used_at: 20,
        };
        let v = serde_json::to_value(&item).unwrap();
        assert_eq!(v["kind"], "link");
        for key in ["imageFile", "sourceApp", "spaceId", "createdAt", "usedAt"] {
            assert!(v.get(key).is_some(), "falta {key}");
        }
        let back: Item = serde_json::from_value(v).unwrap();
        assert_eq!(back, item);
    }

    #[test]
    fn list_filter_accepts_partial_camel_case_json() {
        let f: ListFilter =
            serde_json::from_str(r#"{"view":"space","spaceId":3,"kind":"image"}"#).unwrap();
        assert_eq!(f.view, "space");
        assert_eq!(f.space_id, Some(3));
        assert_eq!(f.kind, Some(Kind::Image));
        assert_eq!(f.query, None);
        let f: ListFilter = serde_json::from_str(r#"{"view":"recent","query":null}"#).unwrap();
        assert_eq!(f, ListFilter::default());
    }

    #[test]
    fn kind_string_round_trip() {
        for k in [
            Kind::Text,
            Kind::Link,
            Kind::Image,
            Kind::Files,
            Kind::Secret,
        ] {
            assert_eq!(Kind::parse(k.as_str()), Some(k));
        }
        assert_eq!(Kind::parse("otra"), None);
    }
}
