use std::path::PathBuf;

use bevy::asset::AssetPath;

/// Resolves a relative asset path. The relative path can be one of:
/// * An absolute path e.g. `foo/bar#fragment`
/// * A path starting with './' or '../', e.g. `./bar#fragment`, in which case it is resolved
///   relative to the current directory.
/// * Just a label, `#fragment`.
pub fn relative_asset_path<'a>(base: &'a AssetPath<'a>, path: &'a str) -> AssetPath<'a> {
    if let Some(label) = path.strip_prefix('#') {
        // It's a label only
        AssetPath::new_ref(&base.path, Some(label))
    } else {
        let (rpath, rlabel) = match path.split_once('#') {
            Some((path, label)) => (path, Some(label.to_string())),
            None => (path, None),
        };
        let mut fpath = PathBuf::from(base.path());
        if !fpath.pop() {
            panic!("Can't compute relative path - not enough path elements");
        }

        let rpath = PathBuf::from(rpath);
        let mut first = true;
        for elt in rpath.iter() {
            if elt == "." {
                // Skip
            } else if elt == ".." {
                if !fpath.pop() {
                    panic!("Can't compute relative path - not enough path elements");
                }
            } else {
                if first {
                    // If the first path element is not '.' or '..' then start fresh.
                    fpath.clear();
                }
                fpath.push(elt);
            }
            first = false;
        }

        AssetPath::new(fpath, rlabel)
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
