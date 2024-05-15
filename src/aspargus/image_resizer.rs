use image::imageops::FilterType;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

/// Resizes an image, that will be overwritten.
/// 
/// ### Parameters
/// - `image_path`: The Aspargus settings.
/// 
/// ### Returns
/// An empty Result in case of success.
/// 
/// ### Errors
/// Returns an error if the resize operation fails.
pub fn resize_image(image_path: &str) -> anyhow::Result<()> {
    const MAX_SIZE: u32 = 672;
    let img = image::open(image_path)?;
    let (width, height) = calculate_new_size(img.width(), img.height(), MAX_SIZE, MAX_SIZE);
    let resized = img.resize_exact(width, height, FilterType::Lanczos3);
    resized.save(image_path)?;
    Ok(())
}

/// Calculates the new size of an image given some boundaries, while keeping the image ratio.
/// 
/// ### Parameters
/// - `width`: The current width of the image.
/// - `height`: The current height of the image.
/// - `max_width`: The maximum width of the image.
/// - `max_height`: The maximum height of the image.
/// 
/// ### Returns
/// A tuple with the new width and height.
fn calculate_new_size(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    let mut new_width = max_width;
    let mut new_height = max_height;
    if width > height {
        new_height = new_width * height / width;
    } else if width < height {
        new_width = new_height * width / height;
    }
    return (new_width, new_height);
}

/// Resizes a list of images.
/// 
/// ### Parameters
/// - `images`: An array of images paths.
pub fn resize_images(images: &Vec<String>) {
    images.par_iter().for_each(|image| {
        let _ = resize_image(image.as_str());
    });
}
