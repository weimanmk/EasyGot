fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let directory = args.next().unwrap_or_else(|| "qa-captures/headless".into());
    let font = args.next().unwrap_or_default();
    easygot::ui::render_samples(
        std::path::Path::new(&directory),
        std::path::Path::new(&font),
    )
}
