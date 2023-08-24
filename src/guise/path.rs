use std::path::PathBuf;

use bevy::asset::AssetPath;

/// Resolves a relative asset path. The relative path can be one of:
/// * An absolute path e.g. `foo/bar#fragment`
/// * A path starting with './' or '../', e.g. `./bar#fragment`, in which case it is resolved
///   relative to the current directory.
/// * Just a label, `#fragment`.
pub fn relative_asset_path<'a>(base: &'a AssetPath<'a>, relative_path: &'a str) -> AssetPath<'a> {
    if relative_path.starts_with('#') {
        AssetPath::new_ref(base.path(), Some(&relative_path[1..]))
    } else if relative_path.starts_with("./") || relative_path.starts_with("../") {
        let mut rpath = relative_path;
        let mut fpath = PathBuf::from(base.path());
        if !fpath.pop() {
            panic!("Can't compute relative path");
        }
        loop {
            if rpath.starts_with("./") {
                rpath = &rpath[2..];
            } else if rpath.starts_with("../") {
                rpath = &rpath[3..];
                if !fpath.pop() {
                    panic!("Can't compute relative path");
                }
            } else {
                break;
            }
        }
        fpath.push(rpath);
        // Note: converting from a string causes AssetPath to look for the '#' separator, while
        // passing fpath directly does not. We want the former.
        AssetPath::from(String::from(fpath.to_str().unwrap()))
    } else {
        AssetPath::from(relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path() {
        let base = AssetPath::from("alice/bob#carol");
        assert_eq!(
            relative_asset_path(&base, "joe/next"),
            AssetPath::from("joe/next")
        );
        assert_eq!(
            relative_asset_path(&base, "#dave"),
            AssetPath::from("alice/bob#dave")
        );
        assert_eq!(
            relative_asset_path(&base, "./martin#dave"),
            AssetPath::from("alice/martin#dave")
        );
        assert_eq!(
            relative_asset_path(&base, "../martin#dave"),
            AssetPath::from("martin#dave")
        );
    }
}
