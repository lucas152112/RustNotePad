use std::collections::HashMap;

/// 單一檔案關聯的描述。 / Represents a single file association entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAssociation {
    pub extension: String,
    pub command: String,
}

/// 管理副檔名至開啟指令的映射。 / Manages extension-to-command mappings.
#[derive(Debug, Clone, Default)]
pub struct FileAssociations {
    map: HashMap<String, String>,
}

impl FileAssociations {
    /// 建立空集合。 / Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// 設定或更新關聯。 / Inserts or updates an association.
    pub fn set(&mut self, extension: impl AsRef<str>, command: impl Into<String>) {
        let ext = normalize_extension(extension.as_ref());
        self.map.insert(ext, command.into());
    }

    /// 取得關聯。 / Retrieves the command for the given extension.
    pub fn get(&self, extension: impl AsRef<str>) -> Option<&str> {
        let ext = normalize_extension(extension.as_ref());
        self.map.get(&ext).map(|s| s.as_str())
    }

    /// 移除關聯。 / Removes an association and returns whether it existed.
    pub fn remove(&mut self, extension: impl AsRef<str>) -> bool {
        let ext = normalize_extension(extension.as_ref());
        self.map.remove(&ext).is_some()
    }

    /// 列舉所有關聯。 / Returns an iterator over all associations.
    pub fn iter(&self) -> impl Iterator<Item = FileAssociation> + '_ {
        self.map.iter().map(|(ext, cmd)| FileAssociation {
            extension: ext.clone(),
            command: cmd.clone(),
        })
    }

    /// 清空集合。 / Clears all associations.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// 目前總數。 / Returns the number of tracked associations.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 是否為空。 / Checks whether no associations are stored.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

fn normalize_extension(raw: &str) -> String {
    raw.trim_start_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_iter() {
        let mut assoc = FileAssociations::default();
        assoc.set(".RS", "rust-tool");
        assoc.set("txt", "less");

        assert_eq!(assoc.get("rs"), Some("rust-tool"));
        assert_eq!(assoc.get(".TXT"), Some("less"));

        let mut entries: Vec<_> = assoc.iter().collect();
        entries.sort_by(|a, b| a.extension.cmp(&b.extension));
        assert_eq!(
            entries,
            vec![
                FileAssociation {
                    extension: "rs".into(),
                    command: "rust-tool".into()
                },
                FileAssociation {
                    extension: "txt".into(),
                    command: "less".into()
                }
            ]
        );
    }

    #[test]
    fn remove_and_clear() {
        let mut assoc = FileAssociations::default();
        assoc.set("md", "markdown-viewer");
        assert!(assoc.remove("MD"));
        assert!(!assoc.remove("md"));
        assoc.set("log", "tail -f");
        assoc.clear();
        assert!(assoc.is_empty());
    }
}
