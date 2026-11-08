//! How a path on disk becomes a path in the model.

use std::path::{Component, Path};

/// Join slash-separated relative paths without requiring the target to exist.
/// Reject absolute paths and traversals outside the project.
pub fn join_relative(base: &str, path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for part in base.split('/').chain(path.split('/')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// The model's spelling of `path`: relative to `root`, separated by `/`.
///
/// Every path in every table goes through here, which is what makes the JSON identical
/// on Windows and on Unix — and, because paths are the tables' sort keys, what makes
/// the *order* of the tables identical too.
///
/// Returns `None` when `path` is not inside `root`, or when a component is not valid
/// UTF-8; callers record that as a diagnostic rather than guessing a spelling.
pub fn relative_slash(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;

    let mut out = String::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return None;
        };
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(part.to_str()?);
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::relative_slash;

    #[test]
    fn a_nested_path_is_spelled_with_forward_slashes_on_every_platform() {
        let root = PathBuf::from("project");
        let file: PathBuf = ["project", "src", "app.test.ts"].iter().collect();

        assert_eq!(
            relative_slash(&root, &file).as_deref(),
            Some("src/app.test.ts")
        );
    }

    #[test]
    fn the_root_itself_is_the_empty_path() {
        assert_eq!(
            relative_slash(Path::new("project"), Path::new("project")).as_deref(),
            Some("")
        );
    }

    #[test]
    fn a_path_outside_the_root_has_no_spelling() {
        let outside: PathBuf = ["elsewhere", "app.ts"].iter().collect();

        assert_eq!(relative_slash(Path::new("project"), &outside), None);
    }
}
