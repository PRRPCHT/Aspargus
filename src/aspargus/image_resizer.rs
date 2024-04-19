use image::imageops::FilterType;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub fn resize_image(image_path: &str) -> anyhow::Result<()> {
    let img = image::open(image_path)?;
    let new_size = calculate_new_size(img.width(), img.height());
    let resized = img.resize_exact(new_size.0, new_size.1, FilterType::Lanczos3);
    resized.save(image_path)?;
    Ok(())
}

fn calculate_new_size(width: u32, height: u32) -> (u32, u32) {
    const MAX_SIZE: u32 = 672;
    let mut new_width = MAX_SIZE;
    let mut new_height = MAX_SIZE;
    if width > height {
        new_height = new_width * height / width;
    } else if width < height {
        new_width = new_height * width / height;
    }
    return (new_width, new_height);
}

pub fn resize_images(images: &Vec<String>) {
    images.par_iter().for_each(|image| {
        let _ = resize_image(image.as_str());
    });
}
