use chrono::Datelike;
use directories::ProjectDirs;
use glob::glob;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::Video;

/// Lists the file paths matching a specific pattern, for retreiving the video thumbnails.
///
/// ### Parameters
/// - `temp_folder`: The temp folder where the images are situated.
/// - `video_id`: The id of the video, which prefixes the thumbnails' file names.
///
/// ### Returns
/// An array of paths to the thumbnails.
pub fn list_matching_files(temp_folder: &str, video_id: &str) -> Vec<String> {
    let mut filename_regex = video_id.to_string();
    filename_regex.push_str("_[0-9]*.png");
    let mut matching_files = Vec::new();
    let pattern = format!("{}/{}", temp_folder, filename_regex);
    // Use the glob library to match files against the pattern
    match glob(&pattern) {
        Ok(paths) => {
            for path in paths.filter_map(Result::ok) {
                if let Some(str_path) = path.to_str() {
                    matching_files.push(str_path.to_string());
                }
            }
        }
        Err(e) => log::error!("Failed to read glob pattern: {}", e),
    }

    matching_files
}

/// Retreives the application's folders, and creates them if they do not exist.
///
/// ### Returns
/// A tuple with the paths to the working folder and the temp folder.
pub fn make_app_folders() -> anyhow::Result<(String, String)> {
    if let Some(proj_dirs) = ProjectDirs::from("ai", "aspargus", "Aspargus") {
        log::debug!("Config dir: {}", proj_dirs.config_dir().to_str().unwrap());
        if Path::new(proj_dirs.config_dir()).is_dir() {
            log::debug!("Config dir exists")
        } else {
            log::debug!("Config dir doesn't exist, let's create it...");
            fs::create_dir_all(proj_dirs.config_dir()).expect("Can't create new config folder");
        }
        let temp_path = proj_dirs.config_dir().join("tmp");
        if Path::new(temp_path.as_path()).is_dir() {
            log::debug!("Tmp dir exists")
        } else {
            log::debug!("Tmp dir doesn't exist, let's create it...");
            fs::create_dir(temp_path.clone()).expect("Can't create new temps folder");
        }
        Ok((
            proj_dirs.config_dir().to_str().unwrap().to_string(),
            temp_path.as_path().to_str().unwrap().to_string(),
        ))
    } else {
        Err(anyhow::Error::msg("No app data folder available"))
    }
}

/// Creates a new path in order to rename a file.
///
/// ### Parameters
/// - `file_path`: The current file path.
/// - `new_name`: The new file name.
///
/// ### Returns
/// The new path.
pub fn create_new_path(file_path: &str, new_name: &str) -> String {
    let the_file_path = Path::new(file_path);
    let parent = the_file_path.parent();
    let extension = the_file_path.extension();
    let mut new_path = PathBuf::new();
    let new_file_name = format!(
        "{}.{}",
        new_name,
        extension.unwrap_or_default().to_str().unwrap_or_default()
    );
    new_path.push(parent.unwrap());
    new_path.push(new_file_name);
    new_path.to_str().unwrap_or(file_path).to_string()
}

/// Gets the file name from the path.
///
/// ### Parameters
/// - `file_path`: The current file path.
///
/// ### Returns
/// The file name.
pub fn get_file_name(file_path: &str) -> String {
    let the_file_path = Path::new(file_path);
    let file_name = the_file_path.file_stem();
    file_name
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_string()
}

/// Renames a file.
///
/// ### Parameters
/// - `original_path`: The current file path.
/// - `new_name`: The new file name.
///
/// ### Returns
/// An empty Result in case of success.
///
/// ### Errors
/// Returns an error if the rename operation fails.
pub fn rename_file(original_path: &str, new_path: &str) -> anyhow::Result<()> {
    match fs::rename(original_path, new_path) {
        Ok(()) => Ok(()),
        Err(_) => Err(anyhow::Error::msg(format!(
            "Could not rename file: {}",
            original_path
        ))),
    }
}

/// Creates a new file name for a video based on a template.
///
/// ### Parameters
/// - `video`: The video to rename.
/// - `template`: The new file name template.
///
/// ### Returns
/// A new file name.
pub fn create_new_file_name(video: &Video, template: &str) -> String {
    let creation_date = video.creation_date;
    let mut new_name = template.to_string();
    new_name = new_name.replace("%Y", creation_date.year().to_string().as_str());
    new_name = new_name.replace("%M", creation_date.format("%m").to_string().as_str());
    new_name = new_name.replace("%D", creation_date.format("%d").to_string().as_str());
    new_name = new_name.replace("%T", &video.resume.title);
    new_name = new_name.replace("%K", &video.resume.keywords.join("-"));
    new_name = new_name.replace("%J", &video.resume.keywords.join(", "));
    new_name = new_name.replace("%F", get_file_name(&video.path).as_str());
    new_name
}