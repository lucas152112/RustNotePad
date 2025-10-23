use std::fs;
use std::io;
use std::path::Path;

/// Writes data atomically by using a temporary sibling file followed by rename.  
/// 以臨時檔案搭配 rename 實現原子寫入。
pub fn write_atomic(path: &Path, data: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}
