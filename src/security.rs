//! Security utilities for input validation and sanitization

/// Sanitize a file path to prevent directory traversal attacks
///
/// This function:
/// - Rejects any path containing `..` (parent directory references)
/// - Rejects paths with null bytes
/// - Removes leading slashes to ensure relative paths
/// - Normalizes multiple slashes
/// - Removes current directory references (`.`)
pub fn sanitize_path(path: &str) -> Result<String, &'static str> {
    // Reject null bytes
    if path.contains('\0') {
        return Err("Invalid path: contains null byte");
    }

    // Reject any path containing parent directory references
    // Check for ".." as a complete path component
    let has_parent_ref = path.split('/').any(|component| component == "..");
    if has_parent_ref {
        return Err("Invalid path: contains parent directory reference (..)");
    }

    // Split path into components and filter out empty and current directory references
    let components: Vec<&str> = path
        .split('/')
        .filter(|component| !component.is_empty() && *component != ".")
        .collect();

    // Reject empty paths after sanitization
    if components.is_empty() {
        return Err("Invalid path: empty after sanitization");
    }

    // Reconstruct safe path
    Ok(components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path_normal() {
        assert_eq!(sanitize_path("file.txt").unwrap(), "file.txt");
        assert_eq!(sanitize_path("folder/file.txt").unwrap(), "folder/file.txt");
        assert_eq!(sanitize_path("a/b/c/file.txt").unwrap(), "a/b/c/file.txt");
    }

    #[test]
    fn test_sanitize_path_traversal() {
        // All paths containing .. should be rejected
        assert!(sanitize_path("../file.txt").is_err());
        assert!(sanitize_path("../../etc/passwd").is_err());
        assert!(sanitize_path("folder/../file.txt").is_err());
        assert!(sanitize_path("../").is_err());
        assert!(sanitize_path("a/b/../c").is_err());

        // Paths with only . should work
        assert_eq!(
            sanitize_path("./folder/./file.txt").unwrap(),
            "folder/file.txt"
        );
    }

    #[test]
    fn test_sanitize_path_slashes() {
        assert_eq!(sanitize_path("/file.txt").unwrap(), "file.txt");
        assert_eq!(sanitize_path("//file.txt").unwrap(), "file.txt");
        assert_eq!(
            sanitize_path("folder//file.txt").unwrap(),
            "folder/file.txt"
        );
        assert_eq!(
            sanitize_path("folder/file.txt/").unwrap(),
            "folder/file.txt"
        );
    }

    #[test]
    fn test_sanitize_path_invalid() {
        assert!(sanitize_path("").is_err());
        assert!(sanitize_path("/").is_err());
        assert!(sanitize_path("..").is_err());
        assert!(sanitize_path("../..").is_err());
        assert!(sanitize_path("file\0.txt").is_err());
    }

    #[test]
    fn test_sanitize_path_edge_cases() {
        assert_eq!(sanitize_path("...txt").unwrap(), "...txt"); // Three dots is valid
        assert_eq!(sanitize_path("..file").unwrap(), "..file"); // Two dots at start is valid as filename
        assert_eq!(sanitize_path("file..").unwrap(), "file.."); // Two dots at end is valid as filename
        assert_eq!(sanitize_path("fi..le").unwrap(), "fi..le"); // Two dots in middle is valid as filename
        assert_eq!(sanitize_path("f.i.l.e").unwrap(), "f.i.l.e"); // Single dots are fine

        // But ".." as a complete path component is rejected
        assert!(sanitize_path("..").is_err());
        assert!(sanitize_path("../").is_err());
        assert!(sanitize_path("/..").is_err());
        assert!(sanitize_path("/../").is_err());
    }

    #[test]
    fn test_sanitize_path_error_messages() {
        assert_eq!(
            sanitize_path("../etc/passwd").unwrap_err(),
            "Invalid path: contains parent directory reference (..)"
        );
        assert_eq!(
            sanitize_path("file\0.txt").unwrap_err(),
            "Invalid path: contains null byte"
        );
        assert_eq!(
            sanitize_path("").unwrap_err(),
            "Invalid path: empty after sanitization"
        );
    }
}
