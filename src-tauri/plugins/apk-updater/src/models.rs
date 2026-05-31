use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidManifest {
    pub version: String,
    #[serde(default)]
    pub version_code: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: String,
    pub url: String,
    pub sha256: String,
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_json() {
        let json = r#"{"version":"0.2.3","versionCode":3,"notes":"x","pubDate":"","url":"https://e/x.apk","sha256":"ab","signature":"sig"}"#;
        let m: AndroidManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.version, "0.2.3");
        assert_eq!(m.version_code, 3);
        assert_eq!(m.url, "https://e/x.apk");
    }
}
