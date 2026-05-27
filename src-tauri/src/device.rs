use std::path::Path;

/// Lädt die persistente Geräte-ID aus `<dir>/device_id.txt` oder erzeugt sie.
/// Wird beim ersten Start einmal gesetzt und danach unverändert weiterverwendet
/// — damit der Sync-Lock dasselbe Gerät über App-Neustarts hinweg erkennt.
pub fn load_or_create(dir: &Path) -> std::io::Result<String> {
    let path = dir.join("device_id.txt");
    if let Ok(s) = std::fs::read_to_string(&path) {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    std::fs::create_dir_all(dir)?;
    let id = uuid::Uuid::new_v4().to_string();
    std::fs::write(&path, &id)?;
    Ok(id)
}

pub fn hostname() -> String {
    gethostname::gethostname().to_string_lossy().into_owned()
}
