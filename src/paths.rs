use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

pub fn portable_key(relative: &str) -> Result<String> {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative.contains('\\')
        || relative.encode_utf16().count() > 230
    {
        bail!("Invalid or overlong relative path: {relative}");
    }
    let mut keys = Vec::new();
    for component in relative.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with([' ', '.'])
            || component.chars().any(|c| c < ' ' || "<>:\"|?*".contains(c))
        {
            bail!("Invalid path component: {component}");
        }
        let stem = component.split('.').next().unwrap().to_uppercase();
        let reserved = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str());
        let port = stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"));
        if reserved || port.is_some_and(|n| n.chars().count() == 1 && "123456789¹²³".contains(n))
        {
            bail!("Reserved Windows filename: {component}");
        }
        keys.push(component.nfc().collect::<String>().to_lowercase());
    }
    if keys[0] == ".card-ingest" {
        bail!("Reserved task storage directory");
    }
    Ok(keys.join("/"))
}

pub fn join(root: &Path, relative: &str) -> Result<PathBuf> {
    portable_key(relative)?;
    crate::platform::reject_link(root)?;
    let mut result = root.to_path_buf();
    for component in relative.split('/') {
        result.push(component);
        crate::platform::reject_link(&result)?;
    }
    Ok(result)
}

pub fn disjoint(
    a: &crate::platform::RootIdentity,
    b: &crate::platform::RootIdentity,
) -> Result<()> {
    if a.volume == b.volume {
        let a = a
            .relative_directory
            .nfc()
            .collect::<String>()
            .to_lowercase();
        let b = b
            .relative_directory
            .nfc()
            .collect::<String>()
            .to_lowercase();
        if a.is_empty()
            || b.is_empty()
            || a == b
            || a.starts_with(&(b.clone() + "/"))
            || b.starts_with(&(a + "/"))
        {
            bail!("Source and destination directories must not overlap");
        }
    }
    Ok(())
}
