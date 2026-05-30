use std::path::Path;

/// Loads the persistent device ID from `<dir>/device_id.txt`, or creates it.
/// Set once on first launch and kept unchanged thereafter — so that the
/// sync lock can recognize the same device across app restarts.
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

/// Masks a hostname for logging. Hostnames are PII (CLAUDE.md), but a short
/// prefix is useful to distinguish devices in a sync-conflict log. Keeps at
/// most the first 3 characters and appends `***`. Empty → "***".
pub fn mask_hostname(hostname: &str) -> String {
    let prefix: String = hostname.chars().take(3).collect();
    format!("{prefix}***")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_hostname_keeps_short_prefix() {
        assert_eq!(mask_hostname("florians-macbook.local"), "flo***");
        assert_eq!(mask_hostname("pc"), "pc***");
        assert_eq!(mask_hostname(""), "***");
    }
}
