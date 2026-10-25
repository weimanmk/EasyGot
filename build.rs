fn main() {
    for style in ["Regular", "Bold"] {
        assert!(
            std::path::Path::new(&format!("design/fonts/EasyGotSansSC-{style}.otf")).exists(),
            "Run python3 scripts/prepare_fonts.py before building"
        );
    }
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new()
            .with_style("material".into())
            .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles),
    )
    .expect("compile Slint interface");
}
