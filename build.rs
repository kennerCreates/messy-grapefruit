fn main() {
    // Embed Windows application manifest for proper DPI awareness.
    // Without this, Windows applies DPI virtualization at 150% scaling,
    // causing mouse click positions to be offset from the actual cursor.
    #[cfg(target_os = "windows")]
    {
        let _ = embed_resource::compile("app.manifest", embed_resource::NONE);
    }
}
