use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UdlDefinition {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_comment: Option<(String, String)>,
    #[serde(default)]
    pub delimiters: Vec<Delimiter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_pattern: Option<String>,
    #[serde(default)]
    pub operators: Vec<String>,
    #[serde(default = "default_case_sensitive")]
    pub case_sensitive: bool,
}

fn default_case_sensitive() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delimiter {
    pub start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(with = "serde_char_option")]
    pub escape: Option<char>,
}

mod serde_char_option {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<char>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(char) => serializer.serialize_some(&char.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<char>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let option = Option::<String>::deserialize(deserializer)?;
        Ok(option.and_then(|value| value.chars().next()))
    }
}

#[derive(Debug, Error)]
pub enum UdlError {
    #[error("failed to decode Notepad++ UDL: {0}")]
    XmlDecode(#[from] quick_xml::DeError),
    #[error("failed to encode Notepad++ UDL: {0}")]
    XmlEncode(#[from] quick_xml::Error),
}

impl UdlDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            identifier: None,
            extensions: Vec::new(),
            keywords: Vec::new(),
            line_comment: None,
            block_comment: None,
            delimiters: Vec::new(),
            number_pattern: None,
            operators: Vec::new(),
            case_sensitive: true,
        }
    }

    pub fn from_notepad_xml(xml: &str) -> Result<Self, UdlError> {
        let user_lang: UserLang = quick_xml::de::from_str(xml)?;

        let mut definition = UdlDefinition::new(user_lang.name);
        definition.identifier = user_lang.lexer;
        definition.extensions = user_lang
            .ext
            .map(|ext| {
                ext.split_whitespace()
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                    .collect()
            })
            .unwrap_or_default();
        definition.case_sensitive = !matches!(
            user_lang.case_ignored.as_deref(),
            Some(value) if value.eq_ignore_ascii_case("yes")
        );

        if let Some(list) = user_lang.keyword_lists {
            let mut keywords = Vec::new();
            for entry in list.keywords {
                keywords.extend(split_keywords(&entry.text));
            }
            definition.keywords = keywords;
        }

        if let Some(comments) = user_lang.comments {
            let mut line_comment = None;
            let mut block_start = None;
            let mut block_end = None;
            for entry in comments.comments {
                match entry.name.as_str() {
                    "Line" => line_comment = entry.value,
                    "Start" => block_start = entry.value,
                    "End" => block_end = entry.value,
                    _ => {}
                }
            }
            definition.line_comment = line_comment;
            if let (Some(start), Some(end)) = (block_start, block_end) {
                definition.block_comment = Some((start, end));
            }
        }

        if let Some(delimiters) = user_lang.delimiters {
            let mut items = Vec::new();
            for entry in delimiters.delimiters {
                if let Some(start) = entry.open {
                    items.push(Delimiter {
                        start,
                        end: entry.close,
                        escape: entry.escape.and_then(|value| value.chars().next()),
                    });
                }
            }
            definition.delimiters = items;
        }

        Ok(definition)
    }

    pub fn to_notepad_xml(&self) -> Result<String, UdlError> {
        let user_lang = UserLang {
            name: self.name.clone(),
            lexer: self.identifier.clone(),
            ext: (!self.extensions.is_empty()).then(|| self.extensions.join(" ")),
            case_ignored: (!self.case_sensitive).then(|| "yes".to_string()),
            comments: comments_to_xml(self),
            keyword_lists: keywords_to_xml(self),
            delimiters: delimiters_to_xml(self),
        };
        let xml = quick_xml::se::to_string(&user_lang)?;
        Ok(xml)
    }
}

fn split_keywords(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn comments_to_xml(definition: &UdlDefinition) -> Option<Comments> {
    if definition.line_comment.is_none() && definition.block_comment.is_none() {
        return None;
    }
    let mut comments = Vec::new();
    if let Some(line) = &definition.line_comment {
        comments.push(CommentEntry {
            name: "Line".into(),
            value: Some(line.clone()),
        });
    }
    if let Some((start, end)) = &definition.block_comment {
        comments.push(CommentEntry {
            name: "Start".into(),
            value: Some(start.clone()),
        });
        comments.push(CommentEntry {
            name: "End".into(),
            value: Some(end.clone()),
        });
    }
    Some(Comments { comments })
}

fn keywords_to_xml(definition: &UdlDefinition) -> Option<KeywordLists> {
    if definition.keywords.is_empty() {
        return None;
    }
    let text = definition.keywords.join(" ");
    Some(KeywordLists {
        keywords: vec![KeywordEntry {
            name: "Keywords1".into(),
            text,
        }],
    })
}

fn delimiters_to_xml(definition: &UdlDefinition) -> Option<Delimiters> {
    if definition.delimiters.is_empty() {
        return None;
    }
    let delimiters = definition
        .delimiters
        .iter()
        .map(|delimiter| DelimiterEntry {
            name: None,
            open: Some(delimiter.start.clone()),
            close: delimiter.end.clone(),
            escape: delimiter.escape.map(|ch| ch.to_string()),
        })
        .collect();
    Some(Delimiters { delimiters })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "UserLang")]
struct UserLang {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@lexer", default)]
    lexer: Option<String>,
    #[serde(rename = "@ext", default)]
    ext: Option<String>,
    #[serde(rename = "@caseIgnored", default)]
    case_ignored: Option<String>,
    #[serde(rename = "Comments", default)]
    comments: Option<Comments>,
    #[serde(rename = "KeywordLists", default)]
    keyword_lists: Option<KeywordLists>,
    #[serde(rename = "Delimiters", default)]
    delimiters: Option<Delimiters>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Comments {
    #[serde(rename = "Comment", default)]
    comments: Vec<CommentEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommentEntry {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@value", default)]
    value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeywordLists {
    #[serde(rename = "Keywords", default)]
    keywords: Vec<KeywordEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeywordEntry {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "$text", default)]
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Delimiters {
    #[serde(rename = "Delimiter", default)]
    delimiters: Vec<DelimiterEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DelimiterEntry {
    #[serde(rename = "@name", default)]
    name: Option<String>,
    #[serde(rename = "@open", default)]
    open: Option<String>,
    #[serde(rename = "@close", default)]
    close: Option<String>,
    #[serde(rename = "@escape", default)]
    escape: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_udl_xml() {
        let original = UdlDefinition {
            name: "Sample".into(),
            identifier: Some("sample".into()),
            extensions: vec!["foo".into(), "bar".into()],
            keywords: vec!["alpha".into(), "beta".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            delimiters: vec![Delimiter {
                start: "\"".into(),
                end: Some("\"".into()),
                escape: Some('\\'),
            }],
            number_pattern: None,
            operators: vec!["+".into()],
            case_sensitive: false,
        };

        let xml = original.to_notepad_xml().unwrap();
        let parsed = UdlDefinition::from_notepad_xml(&xml).unwrap();
        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.extensions, original.extensions);
        assert_eq!(parsed.keywords, original.keywords);
        assert_eq!(parsed.line_comment, original.line_comment);
        assert_eq!(parsed.block_comment, original.block_comment);
        assert_eq!(parsed.delimiters, original.delimiters);
        assert!(!parsed.case_sensitive);
    }
}
