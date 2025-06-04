use super::video::Resume;
use super::{file_management, image_resizer, Video};
use base64::prelude::*;
use chrono::{DateTime, Utc};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::images::Image;
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use regex::Regex;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{fmt, fs};

#[derive(Debug)]
pub(crate) enum VideoDataError {
    FFMpegNotFoundError(String),
    FrameExtractionError(String),
    FFProbeNotFoundError(String),
    MetadataExtractionError(String),
}

impl std::error::Error for VideoDataError {}

impl fmt::Display for VideoDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            VideoDataError::FFMpegNotFoundError(ref _cause) => write!(f, "FFMpeg not found."),
            VideoDataError::FrameExtractionError(ref cause) => {
                write!(f, "Error while extracting the frames for: {}", cause)
            }

            VideoDataError::FFProbeNotFoundError(ref _cause) => write!(f, "FFProbe not found."),
            VideoDataError::MetadataExtractionError(ref cause) => {
                write!(f, "Error while extracting metadata for: {}", cause)
            }
        }
    }
}

/// Extract frames for a video.
///
/// ### Parameters
/// - `temp_folder`: The path of the temp folder to save the thumbnails in.    
/// - `video`: The video that will have thumbnails extracted.
///   
/// ### Returns
/// A Result containing an array of paths to the thumbnails.
///
/// ### Errors
/// Returns an error if FFmpeg can't be run (e.g. not in the path).
pub(crate) fn extract_frames_for_video(
    temp_folder: &str,
    video: &Video,
) -> anyhow::Result<Vec<String>> {
    let mut path: PathBuf = PathBuf::from(temp_folder);
    let mut filename_template = video.id.clone();
    filename_template.push_str("_%04d.png");
    path = path.join(filename_template);
    let mut binding = Command::new("ffmpeg");
    let mut fps = String::from("fps=1/");
    fps.push_str(format!("{}", video.gap).as_str());
    let ffmpeg_command = binding
        .arg("-i")
        .arg(video.path.as_str())
        .arg("-vf")
        .arg(fps)
        .arg(path.to_str().unwrap())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = ffmpeg_command.status();
    if status.is_err() {
        if status.err().unwrap().kind() == ErrorKind::NotFound {
            let error_message = "FFMpeg can't be found, we're stopping here. Please install FFMpeg and FFProbe and make sure they're in the path.".to_string();
            return Err(VideoDataError::FFMpegNotFoundError(error_message).into());
        } else {
            let error_message = format!("Couldn't run FFmpeg for file {}", video.path);
            return Err(VideoDataError::FrameExtractionError(error_message).into());
        }
    }

    let thumbnails = file_management::list_matching_files(temp_folder, video.id.as_str());
    Ok(thumbnails)
}

/// Runs a text model to create a resume of the video file after it's been analysed by the computer vision model.
///
/// ### Parameters
/// - `ollama`: The model prompter for the text model.    
/// - `model`: The name of the model.   
/// - `video`: The video to analyse.   
///
/// ### Returns
/// A Result containing a resume of the video.
///
/// ### Errors
/// Returns an error if the model can't be reached, doesn't exist, or doesn't return a result.
pub(crate) async fn run_resume_model_for_video(
    ollama: &Ollama,
    model: &str,
    video: &Video,
) -> anyhow::Result<Resume> {
    let prompt = "You are a helpful assistant and expert in concise storytelling. The following text tells the story of a video. Please resume that story in 20 words focusing on the person and their action and less on their environment, from that resume please generate a title of maximum 8 words, and make a list of up to 5 keywords that resumes the story, the keywords will include the person on the video if any (e.g. woman, child...). Please format the answer in a json format: {\"title\": <<title>>, \"description\": <<description>>, \"keywords\": <<array of keywords>>}, with no other text at all, only the json result. The story is:";
    if video.story.is_empty() {
        Err(anyhow::anyhow!("No story to resume for : {}", video.path))
    } else {
        let mut resume_prompt = prompt.to_string();
        resume_prompt += video.story.as_str();
        let options = ModelOptions::default().temperature(0.5);
        let res = ollama
            .generate(GenerationRequest::new(model.to_string(), resume_prompt).options(options))
            .await;
        if let Ok(res) = res {
            Ok(serde_json::from_str(res.response.as_str())?)
        } else {
            Err(anyhow::anyhow!(
                "Couldn't generate answer from resume model for file: {}",
                video.path
            ))
        }
    }
}

/// Runs a computer vision model to create a story of the video file based on thumbnails of this video.
///
/// ### Parameters
/// - `ollama`: The model prompter for the computer vision model.    
/// - `model`: The name of the model.   
/// - `video`: The video to analyse.
///   
/// ### Returns
/// A Result containing a story of the video.
///
/// ### Errors
/// Returns an error if the model can't be reached, doesn't exist, or doesn't return a result.
pub(crate) async fn run_computer_vision_model_for_video(
    ollama: &Ollama,
    model: &str,
    video: &Video,
) -> anyhow::Result<String> {
    let prompt = "The following images are part of a video, they tell a story. Please describe that story focusing on the persons and their action and less on their environment.";

    image_resizer::resize_images(&video.thumbnails);
    let mut images = vec![];
    for thumbnail in &video.thumbnails {
        let image_data = match fs::read(thumbnail) {
            Ok(img) => img,
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Couldn't generate answer from computer vision model for file: {}",
                    video.path,
                ));
            }
        };

        // Encode the image data as Base64
        images.push(Image::from_base64(
            BASE64_STANDARD.encode(&image_data).as_str(),
        ))
    }
        let options = ModelOptions::default().temperature(0.5);
    let res = ollama
        .generate(
            GenerationRequest::new(model.to_string(), prompt.to_string())
                .options(options)
                .images(images),
        )
        .await;
    match res {
        Ok(res) => {
            log::debug!("Story: {}", res.response);
            return Ok(res.response);
        }
        Err(err) => {
            log::debug!("Error in run_computer_vision_model_for_video: {}", err); //TODO push the error to the front
            return Err(anyhow::anyhow!(
                "Couldn't generate answer from computer vision model for file: {}",
                video.path
            ));
        }
    }
}

/// Runs a computer vision model to create a resume of the video file based on thumbnails of this video. Note that all the CV models are not able to generate the proper output at once and therefore it will be necessary to run the 2 septs with CV model than text model.
///
/// ### Parameters
/// - `ollama`: The model prompter for the computer vision model.    
/// - `model`: The name of the model.   
/// - `video`: The video to analyse.   
///
/// ### Returns
/// A Result containing a resume of the video.
///
/// ### Errors
/// Returns an error if the model can't be reached, doesn't exist, or doesn't return a result.
pub(crate) async fn run_only_computer_vision_model_for_video(
    ollama: &Ollama,
    model: &str,
    video: &Video,
) -> anyhow::Result<Resume> {
    let prompt = "The following images are part of a video, they tell a story. Please describe that story focusing on the persons and their action and less on their environment. Please resume that story in 20 words focusing on the person and their action and less on their environment, from that resume please generate a title of maximum 8 words, and make a list of up to 5 keywords that resumes the story, the keywords will include the person on the video if any (e.g. woman, child...). Please format the answer in a valid json format: {\"title\": <<title>>, \"description\": <<description>>, \"keywords\": <<array of keywords>>}, with no other text at all, only the json result.";

    image_resizer::resize_images(&video.thumbnails);
    let mut images = vec![];
    for thumbnail in &video.thumbnails {
        let image_data = match fs::read(thumbnail) {
            Ok(img) => img,
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Couldn't generate answer from computer vision model for file: {}",
                    video.path,
                ));
            }
        };

        // Encode the image data as Base64
        images.push(Image::from_base64(
            BASE64_STANDARD.encode(&image_data).as_str(),
        ))
    }
        let options = ModelOptions::default().temperature(0.5);
    let res = ollama
        .generate(
            GenerationRequest::new(model.to_string(), prompt.to_string())
                .options(options)
                .images(images),
        )
        .await;
    match res {
        Ok(res) => {
            match extract_json(&res.response) {
                Some(story) => {
                    log::debug!("Story: {}", story);
                    Ok(serde_json::from_str(story.as_str())?)
                }
                None => Err(anyhow::anyhow!(
                    "Couldn't generate answer from computer vision model for file: {}",
                    video.path
                )),
            }
            //log::debug!("Story: {}", res.response);
            //return Ok(res.response);
        }
        Err(err) => {
            log::debug!("Error in run_computer_vision_model_for_video: {}", err); //TODO push the error to the front
            return Err(anyhow::anyhow!(
                "Couldn't generate answer from computer vision model for file: {}",
                video.path
            ));
        }
    }
}

/// Gets the video's metadata ia FFprobe.
///
/// ### Parameters
/// - `video_path`: The path to the video to analyse.  
///  
/// ### Returns
/// A Result containing the duration of the video and its creation date.
///
/// ### Errors
/// Returns an error if FFprobe can't be run (e.g. not in the path).
pub(crate) fn get_video_metadata(
    video_path: &str,
) -> anyhow::Result<(Option<f32>, Option<DateTime<Utc>>)> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-show_entries")
        .arg("stream_tags=creation_time")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(video_path)
        .output();

    let output = match output {
        Ok(the_output) => the_output,
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                let error_message = "FFProbe can't be found, we're stopping here. Please install FFMpeg and FFProbe and make sure they're in the path.".to_string();
                return Err(VideoDataError::FFProbeNotFoundError(error_message).into());
            } else {
                let error_message = format!("Couldn't run FFmpeg for file {}", video_path);
                return Err(VideoDataError::MetadataExtractionError(error_message).into());
            }
        }
    };

    let binding = String::from_utf8(output.stdout).unwrap();
    let mut set = HashSet::new();
    let metadata: Vec<String> = binding
        .trim()
        .lines()
        .map(|s| s.to_string())
        .filter(|item| set.insert(item.clone()))
        .collect();

    Ok(parse_metadata_to_tuple(metadata))
}

/// Parses the result from FFprobe into something usable.
///
/// ### Parameters
/// - `values`: The raw values from FFprobe.   
///
/// ### Returns
/// A tuple with the duration of the video and its creation date.
fn parse_metadata_to_tuple(values: Vec<String>) -> (Option<f32>, Option<DateTime<Utc>>) {
    let mut float_opt: Option<f32> = None;
    let mut date_opt: Option<DateTime<Utc>> = None;

    for value in values {
        // Try parsing as a DateTime first
        if let Ok(date) = DateTime::parse_from_rfc3339(&value) {
            date_opt = Some(date.to_utc());
        } else if let Ok(num) = value.parse::<f32>() {
            // Not a date, try parsing as a float
            float_opt = Some(num);
        }
    }

    (float_opt, date_opt)
}

/// Gets the gap between two thumbnails extractions in seconds.
///
/// ### Parameters
/// - `duration`: The duration of the video.  
///  
/// ### Returns
/// The gap between two thumbnails extractions in seconds.
pub(crate) fn get_capture_gap(duration: f32) -> i32 {
    let f_gap = duration / 3.0;
    f_gap.floor() as i32
}

/// Extracts the JSON value in the model's result in the case noise is introduced.
///
/// ### Parameters
/// - `input`: The model's result.   
///
/// ### Returns
/// The extracted JSON.
fn extract_json(input: &str) -> Option<String> {
    // This regex looks for the JSON pattern, assuming no curly braces in strings within the JSON
    let re = Regex::new(r"\{(?:[^{}]*|(?R))*\}").unwrap();
    re.find(input).map(|mat| mat.as_str().to_string())
}

/// Gets the names of the models available on an Ollama server.
///
/// ### Parameters
/// - `ollama`: The ollama instance refering to the server to poll.   
///
/// ### Returns
/// A Vec<String> of models names.
pub async fn get_models_for_server(ollama: &Ollama) -> anyhow::Result<Vec<String>> {
    let models =  ollama.list_local_models().await?;
    let model_names: Vec<String> = models.into_iter().map(|m| m.name).collect();
    Ok(model_names)
}