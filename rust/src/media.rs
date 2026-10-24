//! Bounded image decoding shared by paper previews and vision requests.
use anyhow::{ensure, Context, Result};
use std::io::Cursor;

pub fn thumbnail(bytes: &[u8], width: u32, height: u32) -> Result<image::DynamicImage> {
    ensure!(
        bytes.len() <= 30 * 1024 * 1024,
        "图片超过30MB，请压缩后重试"
    );
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits.max_image_width = Some(16000);
    limits.max_image_height = Some(16000);
    reader.limits(limits);
    Ok(reader
        .decode()
        .context("图片尺寸过大或格式损坏，请压缩后重试")?
        .thumbnail(width, height))
}

pub fn preview_path(source: &std::path::Path) -> Result<std::path::PathBuf> {
    let root = source
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("图片路径无效"))?;
    let directory = root.join("previews");
    std::fs::create_dir_all(&directory)?;
    let name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("图片文件名无效"))?;
    let path = directory.join(name).with_extension("png");
    if !path.exists() {
        thumbnail(&std::fs::read(source)?, 700, 1000)?
            .save_with_format(&path, image::ImageFormat::Png)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_dimensions_are_rejected_before_display() {
        let image = image::DynamicImage::new_rgb8(16001, 1);
        let mut out = Cursor::new(vec![]);
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        assert!(thumbnail(&out.into_inner(), 700, 1000).is_err());
    }
    #[test]
    fn small_images_remain_readable_and_keep_aspect_ratio() {
        let image = image::DynamicImage::new_rgb8(400, 200);
        let mut out = Cursor::new(vec![]);
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        let result = thumbnail(&out.into_inner(), 100, 100).unwrap();
        assert_eq!((result.width(), result.height()), (100, 50));
    }
}
