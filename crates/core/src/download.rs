//! A file handed to the person, as sokkel-ui's `lib/download` hands one to the browser: saved where
//! a browser puts its downloads, under a name no file there has yet.

use std::path::PathBuf;

/// Saves `contents` as `name` in the person's Downloads folder (the temporary folder where there is
/// none), numbering it `name (2).csv` and on where a file has the name, as a browser numbers a second
/// download. Returns where it was saved.
pub fn download(name: &str, contents: &str) -> std::io::Result<PathBuf> {
    let folder = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Downloads"))
        .filter(|path| path.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 => (&name[..dot], &name[dot..]),
        _ => (name, ""),
    };
    let mut path = folder.join(name);
    let mut n = 1;
    while path.exists() {
        n += 1;
        path = folder.join(format!("{stem} ({n}){extension}"));
    }
    std::fs::write(&path, contents)?;
    Ok(path)
}
