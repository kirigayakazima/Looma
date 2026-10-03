use std::path::Path;
use looma_core::models::AssetKind;

/// Classifies a file path into an `AssetKind` and an optional MIME type string.
pub fn classify_file(path: &Path) -> (AssetKind, Option<String>) {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return (AssetKind::Other, None),
    };

    match ext.as_str() {
        // Images
        "png" => (AssetKind::Image, Some("image/png".to_string())),
        "jpg" | "jpeg" => (AssetKind::Image, Some("image/jpeg".to_string())),
        "gif" => (AssetKind::Image, Some("image/gif".to_string())),
        "webp" => (AssetKind::Image, Some("image/webp".to_string())),
        "svg" => (AssetKind::Image, Some("image/svg+xml".to_string())),
        "bmp" => (AssetKind::Image, Some("image/bmp".to_string())),
        "ico" => (AssetKind::Image, Some("image/x-icon".to_string())),
        "tiff" | "tif" => (AssetKind::Image, Some("image/tiff".to_string())),
        "avif" => (AssetKind::Image, Some("image/avif".to_string())),
        "heic" | "heif" => (AssetKind::Image, Some("image/heic".to_string())),

        // Videos
        "mp4" => (AssetKind::Video, Some("video/mp4".to_string())),
        "mkv" => (AssetKind::Video, Some("video/x-matroska".to_string())),
        "avi" => (AssetKind::Video, Some("video/x-msvideo".to_string())),
        "mov" => (AssetKind::Video, Some("video/quicktime".to_string())),
        "wmv" => (AssetKind::Video, Some("video/x-ms-wmv".to_string())),
        "flv" => (AssetKind::Video, Some("video/x-flv".to_string())),
        "webm" => (AssetKind::Video, Some("video/webm".to_string())),
        "m4v" => (AssetKind::Video, Some("video/x-m4v".to_string())),
        "m2ts" | "mts" => (AssetKind::Video, Some("video/mp2t".to_string())),

        // Audio
        "mp3" => (AssetKind::Audio, Some("audio/mpeg".to_string())),
        "wav" => (AssetKind::Audio, Some("audio/wav".to_string())),
        "flac" => (AssetKind::Audio, Some("audio/flac".to_string())),
        "aac" => (AssetKind::Audio, Some("audio/aac".to_string())),
        "ogg" => (AssetKind::Audio, Some("audio/ogg".to_string())),
        "m4a" => (AssetKind::Audio, Some("audio/mp4".to_string())),
        "wma" => (AssetKind::Audio, Some("audio/x-ms-wma".to_string())),
        "opus" => (AssetKind::Audio, Some("audio/opus".to_string())),

        // Documents
        "pdf" => (AssetKind::Document, Some("application/pdf".to_string())),
        "docx" => (AssetKind::Document, Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string())),
        "doc" => (AssetKind::Document, Some("application/msword".to_string())),
        "xlsx" => (AssetKind::Document, Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string())),
        "xls" => (AssetKind::Document, Some("application/vnd.ms-excel".to_string())),
        "pptx" => (AssetKind::Document, Some("application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string())),
        "ppt" => (AssetKind::Document, Some("application/vnd.ms-powerpoint".to_string())),
        "txt" => (AssetKind::Document, Some("text/plain".to_string())),
        "md" | "markdown" => (AssetKind::Document, Some("text/markdown".to_string())),
        "csv" => (AssetKind::Document, Some("text/csv".to_string())),
        "epub" => (AssetKind::Document, Some("application/epub+zip".to_string())),

        // Archives
        "zip" => (AssetKind::Archive, Some("application/zip".to_string())),
        "tar" => (AssetKind::Archive, Some("application/x-tar".to_string())),
        "gz" => (AssetKind::Archive, Some("application/gzip".to_string())),
        "7z" => (AssetKind::Archive, Some("application/x-7z-compressed".to_string())),
        "rar" => (AssetKind::Archive, Some("application/vnd.rar".to_string())),
        "bz2" => (AssetKind::Archive, Some("application/x-bzip2".to_string())),
        "xz" => (AssetKind::Archive, Some("application/x-xz".to_string())),

        // 3D Models
        "obj" => (AssetKind::Model3d, Some("model/obj".to_string())),
        "fbx" => (AssetKind::Model3d, Some("application/octet-stream".to_string())),
        "gltf" => (AssetKind::Model3d, Some("model/gltf+json".to_string())),
        "glb" => (AssetKind::Model3d, Some("model/gltf-binary".to_string())),
        "stl" => (AssetKind::Model3d, Some("model/stl".to_string())),
        "blend" => (AssetKind::Model3d, Some("application/x-blender".to_string())),

        // Code
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "json" | "toml" | "yaml" | "yml" |
        "html" | "css" | "scss" | "sql" | "sh" | "ps1" | "bat" | "c" | "cpp" | "h" |
        "hpp" | "java" | "go" | "kt" | "swift" | "php" | "rb" | "dart" | "zig" | "lua" => {
            let mime = match ext.as_str() {
                "rs" => "text/x-rust",
                "ts" | "tsx" => "text/typescript",
                "js" | "jsx" => "text/javascript",
                "py" => "text/x-python",
                "json" => "application/json",
                "toml" => "text/x-toml",
                "yaml" | "yml" => "text/yaml",
                "html" => "text/html",
                "css" | "scss" => "text/css",
                "sql" => "application/sql",
                "sh" => "application/x-sh",
                _ => "text/plain",
            };
            (AssetKind::Code, Some(mime.to_string()))
        }

        _ => (AssetKind::Other, None),
    }
}
