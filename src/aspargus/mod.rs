use self::settings::AspargusSettings;
use anyhow;
use aspargus_helper::VideoDataError;
use ollama_rs::Ollama;
use rayon::prelude::*;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use video::Video;
use std::fmt;
mod aspargus_helper;
mod file_management;
mod image_resizer;
mod settings;
mod video;

/// Represents an Aspargus error.
#[derive(Debug)]
pub enum AspargusError {
    Io(String),
    ParseError(String),
    GenericError(String),
    ProcessingError(String),
}

impl fmt::Display for AspargusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AspargusError::Io(msg) => write!(f, "IO error: {}", msg),
            AspargusError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            AspargusError::GenericError(msg) => write!(f, "Generic error: {}", msg),
            AspargusError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
        }
    }
}

impl std::error::Error for AspargusError {}

/// Represents an Aspargus instance.
///
/// ### Fields
/// - `videos`: An array of videos to be analysed.
/// - `settings`: The Aspargus settings loaded from a file.
/// - `cv_ollama`: The computer vision model prompter.
/// - `text_ollama`: The text model prompter.
/// - `videos_number`: The number of videos in the queue.
pub struct Aspargus {
    videos: Vec<Video>,
    settings: AspargusSettings,
    cv_ollama: Ollama,
    text_ollama: Ollama,
    videos_number: i32,
}

impl Aspargus {
    /// Creates a new Aspargus instance and creates the work folders/new settings file if needed. It also loads the Aspargus settings.
    /// ### Returns
    /// A new Aspargus instance.
    pub fn new() -> Self {
        let settings = settings::load_settings();
        let computer_vision_server = settings.computer_vision_server.clone();
        let computer_vision_server_port = settings.computer_vision_server_port.clone();
        let text_server = settings.text_server.clone();
        let text_server_port = settings.text_server_port.clone();
        log::debug!("Temp folder: {}", settings.temp_folder);
        Self {
            videos: Vec::new(),
            settings,
            cv_ollama: Ollama::new(computer_vision_server, computer_vision_server_port),
            text_ollama: Ollama::new(text_server, text_server_port),
            videos_number: 0,
        }
    }

    /// Sets the computer vision model name. This name can be obtain by running '''ollama list'''.
    /// ### Parameters
    /// - `model`: The name of the computer vision model.
    pub fn set_computer_vision_model(&mut self, model: String) {
        if self.settings.computer_vision_model != model {
            self.settings.computer_vision_model = model;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    /// Sets the text model name. This name can be obtain by running '''ollama list'''.
    /// ### Parameters
    /// - `model`: The name of the text model.
    pub fn set_text_model(&mut self, model: String) {
        if self.settings.text_model != model {
            self.settings.text_model = model;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    /// Sets the computer vision server address.
    /// ### Parameters
    /// - `server`: The IP of the computer vision server.
    pub fn set_computer_vision_server(&mut self, server: String) {
        if self.settings.computer_vision_server != server {
            self.settings.computer_vision_server = server;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    /// Sets the computer vision server port.
    /// ### Parameters
    /// - `server`: The port of the computer vision server.
    pub fn set_computer_vision_server_port(&mut self, port: u16) {
        if self.settings.computer_vision_server_port != port {
            self.settings.computer_vision_server_port = port;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    /// Sets the text server address.
    /// ### Parameters
    /// - `server`: The IP of the text server.
    pub fn set_text_server(&mut self, server: String) {
        if self.settings.text_server != server {
            self.settings.text_server = server;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    /// Sets the textserver port.
    /// ### Parameters
    /// - `server`: The port of the text server.
    pub fn set_text_server_port(&mut self, port: u16) {
        if self.settings.text_server_port != port {
            self.settings.text_server_port = port;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    /// Sets the two steps flag.
    /// ### Parameters
    /// - `two_steps`: The two steps flag.
    pub fn set_two_steps(&mut self, two_steps: bool) {
        if self.settings.two_steps != two_steps {
            self.settings.two_steps = two_steps;
            match settings::save_settings(&self.settings) {
                Ok(_) => (),
                Err(error) => log::error!("{}", error),
            }
        }
    }

    pub fn is_two_steps(&mut self) -> bool {
        self.settings.two_steps
    }

    /// Add a whole list of videos to be analysed to Aspargus.
    /// ### Parameters
    /// - `paths`: The paths of the videos to analyse.
    pub fn add_videos(&mut self, paths: Vec<String>) -> Result<(), AspargusError> { 
        for path in paths {
            match self.add_video(path) {
                Ok(_) => self.videos_number += 1,
                Err(error) => {
                    log::error!("Error while adding video: {}", error);
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    /// Gets a new numeric ID for a video.
    /// ### Returns
    /// A new numeric ID for a video.
    fn get_new_video_numeric_id(&mut self) -> i32 {
        if self.videos.len() >= 1 {
            self.videos.last().unwrap().numeric_id + 1
        } else {
            1
        }
    }

    /// Gets the name of the currently set computer vision model.
    /// ### Returns
    /// The name of the currently set computer vision model.
    pub fn get_computer_vision_model(&self) -> String {
        self.settings.computer_vision_model.clone()
    }

    /// Gets the list of computer vision models available on the server.
    /// ### Returns
    /// A list of computer vision models available on the server.
    pub async fn get_computer_vision_models_list(&self) -> Result<Vec<String>, AspargusError> { 
        match aspargus_helper::get_models_for_server(&self.cv_ollama).await {
            Ok(models) => Ok(models),
            Err(error) => {
                log::error!("Error while getting computer vision models list: {}", error);
                Err(AspargusError::Io(format!(
                    "Error while getting computer vision models list: {}",
                    error
                )))
            }
        }
    }

    /// Gets the name of the currently set text model.
    /// ### Returns
    /// The name of the currently set text model.
    pub fn get_text_model(&self) -> String {
        self.settings.text_model.clone()
    }

    /// Gets the list of text models available on the server.
    /// ### Returns
    /// A list of text models available on the server.
    pub async fn get_text_models_list(&self) -> Result<Vec<String>, AspargusError> { 
        match aspargus_helper::get_models_for_server(&self.text_ollama).await {
            Ok(models) => Ok(models),
            Err(error) => {
                log::error!("Error while getting text models list: {}", error);
                Err(AspargusError::Io(format!(
                    "Error while getting text models list: {}",
                    error
                )))
            }
        }
    }

    /// Add a video to be analysed to Aspargus.
    /// ### Parameters
    /// - `path`: The path of the video to analyse.
    pub fn add_video(&mut self, path: String) -> Result<(), AspargusError> {
        let the_path = Path::new(path.as_str());
        if the_path.is_file() {
            match Video::new(path.clone(), self.get_new_video_numeric_id()) {
                Ok(video) => self.videos.push(video),
                Err(error) => {
                    if let Some(metadata_extraction_error) = error.downcast_ref::<VideoDataError>()
                    {
                        match metadata_extraction_error {
                        VideoDataError::FFProbeNotFoundError(_) => return Err(AspargusError::GenericError("FFProbe is not found, we're quitting for now. Please install FFMpeg and FFProbe and put them in the path.".to_string())),
                            VideoDataError::FrameExtractionError(_) => log::error!("Error while extracting metadata for: {}, it won't be processed further on.", path),
                            _ => (), // Other cases are not for frame extraction
                        }
                    } else {
                        log::error!("Error while extracting metadata for: {}, it won't be processed further on.", &path);
                        return Err(AspargusError::ProcessingError(format!("Error while extracting metadata for: {}", &path)))
                    }
                }
            }
        } else {
            log::error!(
                "File {} doesn't exist or is not a file, and therefore will be ignored.",
                path
            );
            return Err(AspargusError::ProcessingError(format!(
                "File {} doesn't exist or is not a file, and therefore will be ignored.",
                path
            )));
        }
        Ok(())
    }

    /// Extract frames for all the videos in the list in the Aspargus struct.
    pub fn extract_frames(&mut self) -> Result<(), AspargusError> { 
        let error_holder = Arc::new(Mutex::new(None));
        self.videos.par_iter_mut().for_each(|video| {
            log::info!(
                "{}/{} - Extracting frames for {}",
                video.numeric_id,
                self.videos_number,
                video.path
            );
            match aspargus_helper::extract_frames_for_video(self.settings.temp_folder.as_str(), video) {
                Ok(thumbnails) => {
                    video.thumbnails = thumbnails;
                    //extract_faces_from_thumbnails(thumbnails);
                }
                Err(error) =>  {
                    if let Some(extraction_error) = error.downcast_ref::<VideoDataError>() {
                        match extraction_error {
                            VideoDataError::FFMpegNotFoundError(_) => {
                                let mut holder = error_holder.lock().unwrap();
                                if holder.is_none() { // Only capture the first error
                                    *holder = Some(anyhow::anyhow!("FFMpeg is not found, we're quitting for now. Please install FFMpeg and FFProbe and put them in the path."));
                                }
                            },
                            VideoDataError::FrameExtractionError(_) => {
                                video.skip = true;
                                log::error!("{}/{} - Error while extracting frames for: {}, it won't be processed further on.", video.numeric_id, self.videos_number, error)
                            },
                            _ => (), // Other cases are not for frame extraction
                        }
                    } else {
                        log::error!("{}/{} - Error while extracting frames for: {}, it won't be processed further on.", video.numeric_id, self.videos_number, error)
                    }
                },
            }
        });
        let mut locked_error: std::sync::MutexGuard<Option<anyhow::Error>> =
            error_holder.lock().unwrap();
        if let Some(err) = locked_error.take() {
            Err(AspargusError::ProcessingError(format!("Error while extracting frames: {}", err.to_string())))
        } else {
            Ok(())
        }
    }

    /// Runs the computer vision model for all the videos files. Note that this method must be run before the '''run_resume_model''' method.
    pub async fn run_computer_vision_model(&mut self) {
        for video in &mut self.videos {
            if video.skip {
                log::info!(
                    "{}/{} - Skipping {}",
                    video.numeric_id,
                    self.videos_number,
                    video.path
                );
            } else {
                log::info!(
                    "{}/{} - Running computer vision model for {}",
                    video.numeric_id,
                    self.videos_number,
                    video.path
                );
                match aspargus_helper::run_computer_vision_model_for_video(
                    &self.cv_ollama,
                    &self.settings.computer_vision_model,
                    video,
                )
                .await
                {
                    Ok(story) => video.story = story,
                    Err(error) => log::error!(
                        "{}/{} - Error while running computer vision model: {}",
                        video.numeric_id,
                        self.videos_number,
                        error
                    ),
                }
            }
        }
    }

    /// Runs the computer vision model for all the videos files that is able to provide a full result without running the second step with the resume model.
    pub async fn run_only_computer_vision_model(&mut self) {
        for video in &mut self.videos {
            if video.skip {
                log::info!(
                    "{}/{} - Skipping {}",
                    video.numeric_id,
                    self.videos_number,
                    video.path
                );
            } else {
                log::info!(
                    "{}/{} - Running computer vision model for {}",
                    video.numeric_id,
                    self.videos_number,
                    video.path
                );
                match aspargus_helper::run_only_computer_vision_model_for_video(
                    &self.cv_ollama,
                    &self.settings.computer_vision_model,
                    video,
                )
                .await
                {
                    Ok(resume) => video.resume = resume,
                    Err(error) => log::error!(
                        "{}/{} - Error while running computer vision model: {}",
                        video.numeric_id,
                        self.videos_number,
                        error
                    ),
                }
            }
        }
    }

    /// Runs the text model for all the videos files based on the computer vision model's output.
    pub async fn run_resume_model(&mut self) {
        for video in &mut self.videos {
            if video.skip {
                log::info!(
                    "{}/{} - Skipping {}",
                    video.numeric_id,
                    self.videos_number,
                    video.path
                );
            } else {
                log::info!(
                    "{}/{} - Running resume model for {}",
                    video.numeric_id,
                    self.videos_number,
                    video.path
                );
                match aspargus_helper::run_resume_model_for_video(
                    &self.text_ollama,
                    &self.settings.text_model,
                    video,
                )
                .await
                {
                    Ok(resume) => {
                        log::info!(
                            "{}/{} - Title: {}",
                            video.numeric_id,
                            self.videos_number,
                            resume.title
                        );
                        log::info!(
                            "{}/{} - Description: {}",
                            video.numeric_id,
                            self.videos_number,
                            resume.description
                        );
                        log::info!(
                            "{}/{} - Keywords: {}",
                            video.numeric_id,
                            self.videos_number,
                            resume.keywords.join(", ")
                        );
                        video.resume = resume;
                    }
                    Err(error) => log::error!(
                        "{}/{} - Error while running resume model: {}",
                        video.numeric_id,
                        self.videos_number,
                        error
                    ),
                }
            }
        }
    }

    /// Exports the results of the analysis in a JSON file.
    ///
    /// ### Parameters
    /// - `path`: The path of the file to write.  
    ///   
    /// ### Returns
    /// An empty Result in case of success.
    ///
    /// ### Errors
    /// Returns an error if the export fails.
    pub fn export_to_json(&self, path: &str) -> Result<(), AspargusError> { 
        let contents = match serde_json::to_string_pretty(&self.videos) {
            Ok(json) => json,
            Err(_) => {
                return Err(AspargusError::GenericError(
                    "Error while serializing the videos to JSON".to_string(),
                ))
            }
        };
        match fs::write(path, contents) {
            Ok(_) => {
                log::info!("Exported results to {}", path);
            }
            Err(error) => {
                log::error!("Error while exporting results to JSON: {}", error);
                return Err(AspargusError::Io(format!(
                    "Error while exporting results to JSON: {}",
                    error
                )));
            }
        };
        Ok(())
    }

    /// Renames the videos based on the results of the analysis.
    ///
    /// ### Parameters
    /// - `template`: The template for the new file name.
    pub fn rename_videos(&mut self, template: &str) {
        self.videos.par_iter_mut().for_each(|video| {
            let new_name = file_management::create_new_file_name(video, template);
            let new_path =
                &file_management::create_new_path(video.path.as_str(), new_name.as_str());
            match file_management::rename_file(&video.path, new_path) {
                Ok(_) => log::info!(
                    "{}/{} - Renamed to: {}",
                    video.numeric_id,
                    self.videos_number,
                    new_name
                ),
                Err(error) => log::error!(
                    "{}/{} - Error while renaming file: {}",
                    video.numeric_id,
                    self.videos_number,
                    error
                ),
            }
        });
    }

/// Filters the content of a directory based on a start and end file namen (alphabetically).
///
/// ### Parameters
/// - `dir_path`: The path of the directory.
/// - `file_name_start`: The first file to be selected, None if we start from the beginning.
/// - `file_name_end`: TThe last file to be selected, None if we finish at the end.
///
/// ### Returns
/// A list of file paths. If the directory doesn't exist or if it is empty, an empty list is returned.
///
pub fn filter_files_in_dir(
    dir_path: &PathBuf,
    file_name_start: Option<&str>,
    file_name_end: Option<&str>,
) -> Vec<String> {
    file_management::filter_files_in_dir(dir_path, file_name_start, file_name_end)
}

}
