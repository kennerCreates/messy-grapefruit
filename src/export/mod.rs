pub mod svg_gen;
pub mod rasterize;
pub mod bone_export;
pub mod ron_meta;
pub mod spritesheet;
pub mod watcher;

use std::path::Path;

/// High-level export error type.
#[derive(Debug)]
pub enum ExportError {
    Io(std::io::Error),
    #[allow(dead_code)]
    Svg(String),
    Rasterize(String),
    Ron(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "IO error: {}", e),
            ExportError::Svg(e) => write!(f, "SVG generation error: {}", e),
            ExportError::Rasterize(e) => write!(f, "Rasterization error: {}", e),
            ExportError::Ron(e) => write!(f, "RON export error: {}", e),
        }
    }
}

impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        ExportError::Io(e)
    }
}

/// Ensure a directory exists, creating it if necessary.
pub fn ensure_dir(path: &Path) -> Result<(), ExportError> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}
