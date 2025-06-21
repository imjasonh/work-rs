#[cfg(test)]
mod r2_storage_tests {
    use crate::r2_storage::*;
    use std::collections::HashMap;
    
    // Mock R2 bucket for testing
    struct MockR2Bucket {
        files: HashMap<String, (Vec<u8>, Option<String>)>,
    }
    
    impl MockR2Bucket {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
            }
        }
        
        fn put(&mut self, key: &str, data: Vec<u8>, content_type: Option<String>) {
            self.files.insert(key.to_string(), (data, content_type));
        }
        
        fn get(&self, key: &str) -> Option<(Vec<u8>, Option<String>)> {
            self.files.get(key).cloned()
        }
        
        fn delete(&mut self, key: &str) -> bool {
            self.files.remove(key).is_some()
        }
        
        fn list(&self, prefix: Option<&str>) -> Vec<String> {
            self.files
                .keys()
                .filter(|k| {
                    if let Some(p) = prefix {
                        k.starts_with(p)
                    } else {
                        true
                    }
                })
                .cloned()
                .collect()
        }
    }
    
    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata {
            key: "test.txt".to_string(),
            size: 1024,
            content_type: Some("text/plain".to_string()),
            uploaded_at: 1234567890,
        };
        
        assert_eq!(metadata.key, "test.txt");
        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.content_type, Some("text/plain".to_string()));
        assert_eq!(metadata.uploaded_at, 1234567890);
    }
    
    #[test]
    fn test_r2_upload_operations() {
        let mut bucket = MockR2Bucket::new();
        
        // Test upload with content type
        bucket.put("file1.txt", vec![1, 2, 3], Some("text/plain".to_string()));
        assert!(bucket.get("file1.txt").is_some());
        
        // Test upload without content type
        bucket.put("file2.bin", vec![4, 5, 6], None);
        let (data, content_type) = bucket.get("file2.bin").unwrap();
        assert_eq!(data, vec![4, 5, 6]);
        assert_eq!(content_type, None);
        
        // Test overwrite
        bucket.put("file1.txt", vec![7, 8, 9], Some("text/plain".to_string()));
        let (data, _) = bucket.get("file1.txt").unwrap();
        assert_eq!(data, vec![7, 8, 9]);
    }
    
    #[test]
    fn test_r2_download_operations() {
        let mut bucket = MockR2Bucket::new();
        
        // Test download existing file
        bucket.put("test.txt", vec![1, 2, 3], Some("text/plain".to_string()));
        let result = bucket.get("test.txt");
        assert!(result.is_some());
        let (data, content_type) = result.unwrap();
        assert_eq!(data, vec![1, 2, 3]);
        assert_eq!(content_type, Some("text/plain".to_string()));
        
        // Test download non-existent file
        let result = bucket.get("missing.txt");
        assert!(result.is_none());
    }
    
    #[test]
    fn test_r2_delete_operations() {
        let mut bucket = MockR2Bucket::new();
        
        // Test delete existing file
        bucket.put("test.txt", vec![1, 2, 3], None);
        assert!(bucket.delete("test.txt"));
        assert!(bucket.get("test.txt").is_none());
        
        // Test delete non-existent file
        assert!(!bucket.delete("missing.txt"));
    }
    
    #[test]
    fn test_r2_list_operations() {
        let mut bucket = MockR2Bucket::new();
        
        // Setup test files
        bucket.put("file1.txt", vec![], None);
        bucket.put("images/photo1.jpg", vec![], None);
        bucket.put("images/photo2.png", vec![], None);
        bucket.put("docs/readme.md", vec![], None);
        
        // Test list all
        let all_files = bucket.list(None);
        assert_eq!(all_files.len(), 4);
        
        // Test list with prefix
        let mut images = bucket.list(Some("images/"));
        images.sort();
        assert_eq!(images, vec!["images/photo1.jpg", "images/photo2.png"]);
        
        // Test list with non-existent prefix
        let videos = bucket.list(Some("videos/"));
        assert!(videos.is_empty());
    }
    
    #[test]
    fn test_content_type_inference() {
        let test_cases = vec![
            ("file.txt", "text/plain"),
            ("file.html", "text/html"),
            ("file.json", "application/json"),
            ("file.jpg", "image/jpeg"),
            ("file.jpeg", "image/jpeg"),
            ("file.png", "image/png"),
            ("file.gif", "image/gif"),
            ("file.pdf", "application/pdf"),
            ("file.unknown", "application/octet-stream"),
            ("file", "application/octet-stream"),
        ];
        
        for (filename, expected_type) in test_cases {
            let inferred = match filename.split('.').last() {
                Some("txt") => "text/plain",
                Some("html") => "text/html",
                Some("json") => "application/json",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("png") => "image/png",
                Some("gif") => "image/gif",
                Some("pdf") => "application/pdf",
                _ => "application/octet-stream",
            };
            assert_eq!(inferred, expected_type, "Failed for file: {}", filename);
        }
    }
}