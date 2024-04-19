use chrono::Datelike;
use directories::ProjectDirs;
use glob::glob;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::Video;

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

pub fn get_file_name(file_path: &str) -> String {
    let the_file_path = Path::new(file_path);
    let file_name = the_file_path.file_stem();
    file_name
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_string()
}

pub fn rename_file(original_path: &str, new_path: &str) -> anyhow::Result<()> {
    match fs::rename(original_path, new_path) {
        Ok(()) => Ok(()),
        Err(_) => Err(anyhow::Error::msg(format!(
            "Could not rename file: {}",
            original_path
        ))),
    }
}

pub fn create_new_file_name(video: &Video, template: &str) -> String {
    let creation_date = video.creation_date;
    // let title = "Zsolna fait du vélo";
    // let keywords = [
    //     "zsolna".to_string(),
    //     "vélo".to_string(),
    //     "extérieur".to_string(),
    // ];
    //let file = "/Users/pierre/thevideo.mp4";
    //let template = "%Y-%M-%D_%T_%K".to_string();
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

pub fn filter_files_in_dir(
    dir_path: &PathBuf,
    file_name_start: Option<&str>,
    file_name_end: Option<&str>,
) -> Vec<String> {
    let path = Path::new(dir_path);
    let mut filtered_paths = Vec::new();

    // Check if the path exists and is a directory
    if path.exists() && path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    let file_name: Option<String> = match path_to_string(&path) {
                        Ok(the_file_name) => Some(the_file_name),
                        Err(_) => {
                            log::error!("This {:?} will be ignored due to an error", path);
                            None
                        }
                    };
                    if file_name.is_some() {
                        let file_name = file_name.unwrap();
                        let file_name = file_name.as_str();
                        // Check if the file name matches the start and end constraints
                        let matches_start = file_name_start
                            .map(|start| file_name >= start)
                            .unwrap_or(true); // If no start constraint, always true

                        let matches_end = file_name_end.map(|end| file_name <= end).unwrap_or(true); // If no end constraint, always true

                        if matches_start && matches_end {
                            if let Some(path_str) = path.to_str() {
                                filtered_paths.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    filtered_paths
}

fn path_to_string(path: &PathBuf) -> anyhow::Result<String> {
    if let Some(file_name_inter) = path.file_name() {
        if let Some(file_name) = file_name_inter.to_str() {
            Ok(file_name.to_string())
        } else {
            Err(anyhow::anyhow!("No story to resume for : {:?}", path))
        }
    } else {
        Err(anyhow::anyhow!("No story to resume for : {:?}", path))
    }
}
