use anyhow;
use chksum_hash_md5 as md5;
use chrono::{DateTime, Utc};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::generation::images::Image;
use ollama_rs::generation::options::GenerationOptions;
use ollama_rs::Ollama;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
mod image_resizer;
use base64::prelude::*;
use rayon::prelude::*;

use self::settings::AspargusSettings;
pub(crate) mod file_management;
mod settings;

#[derive(Default, Deserialize, Serialize, Debug)]
struct Resume {
    title: String,
    description: String,
    keywords: Vec<String>,
}

#[derive(Default, Serialize)]

pub struct Video {
    #[serde(skip_serializing)]
    id: String,
    path: String,
    #[serde(skip_serializing)]
    story: String,
    resume: Resume,
    #[serde(skip_serializing)]
    thumbnails: Vec<String>,
    #[serde(skip_serializing)]
    creation_date: DateTime<Utc>,
    #[serde(skip_serializing)]
    gap: i32,
    #[serde(skip_serializing)]
    numeric_id: i32,
}

impl Video {
    pub fn new(path: String, numeric_id: i32) -> Self {
        let id = md5::hash(&path).to_hex_lowercase();
        let (duration, creation_date) = get_video_metadata(path.as_str()).unwrap_or_default();
        let gap = get_capture_gap(duration.unwrap_or_default());
        Self {
            id,
            path,
            story: String::default(),
            resume: Resume::default(),
            thumbnails: Vec::new(),
            creation_date: creation_date.unwrap_or_default(),
            gap,
            numeric_id,
        }
    }
}

pub(crate) struct Aspargus {
    videos: Vec<Video>,
    settings: AspargusSettings,
    cv_ollama: Ollama,
    text_ollama: Ollama,
    videos_number: i32,
}

impl Aspargus {
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

    pub fn set_computer_vision_model(&mut self, model: String) {
        self.settings.computer_vision_model = model;
        match settings::save_settings(&self.settings) {
            Ok(_) => (),
            Err(error) => log::error!("{}", error),
        }
    }

    pub fn set_text_model(&mut self, model: String) {
        self.settings.text_model = model;
        match settings::save_settings(&self.settings) {
            Ok(_) => (),
            Err(error) => log::error!("{}", error),
        }
    }

    pub fn set_computer_vision_server(&mut self, server: String) {
        self.settings.computer_vision_server = server;
        match settings::save_settings(&self.settings) {
            Ok(_) => (),
            Err(error) => log::error!("{}", error),
        }
    }

    pub fn set_text_server(&mut self, server: String) {
        self.settings.text_server = server;
        match settings::save_settings(&self.settings) {
            Ok(_) => (),
            Err(error) => log::error!("{}", error),
        }
    }

    pub fn add_videos(&mut self, paths: Vec<String>) {
        for path in paths {
            self.add_video(path);
            self.videos_number += 1;
        }
    }

    fn get_new_video_numeric_id(&mut self) -> i32 {
        if self.videos.len() >= 1 {
            self.videos.last().unwrap().numeric_id + 1
        } else {
            1
        }
    }

    pub fn add_video(&mut self, path: String) {
        let video = Video::new(path, self.get_new_video_numeric_id());
        self.videos.push(video);
    }

    pub fn extract_frames(&mut self) {
        self.videos.par_iter_mut().for_each(|video| {
            log::info!(
                "{}/{} - Extracting frames for {}",
                video.numeric_id,
                self.videos_number,
                video.path
            );
            match extract_frames_for_video(self.settings.temp_folder.as_str(), video) {
                Ok(thumbnails) => {
                    video.thumbnails = thumbnails;
                    //extract_faces_from_thumbnails(thumbnails);
                }
                Err(error) => log::error!(
                    "{}/{} - Error while extracting frames: {}",
                    video.numeric_id,
                    self.videos_number,
                    error
                ),
            }
        });
    }

    pub async fn run_computer_vision_model(&mut self) {
        for video in &mut self.videos {
            log::info!(
                "{}/{} - Running computer vision model for {}",
                video.numeric_id,
                self.videos_number,
                video.path
            );
            match run_computer_vision_model_for_video(
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

    pub async fn run_resume_model(&mut self) {
        for video in &mut self.videos {
            log::info!(
                "{}/{} - Running resume model for {}",
                video.numeric_id,
                self.videos_number,
                video.path
            );
            match run_resume_model_for_video(&self.text_ollama, &self.settings.text_model, video)
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

    pub fn export_to_json(&self, path: &str) -> anyhow::Result<()> {
        let contents = serde_json::to_string_pretty(&self.videos)?;
        let _ = fs::write(path, contents)?;
        Ok(())
    }

    pub fn rename_videos(&mut self, template: &str) -> anyhow::Result<()> {
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
        Ok(())
    }
}

fn extract_frames_for_video(temp_folder: &str, video: &Video) -> anyhow::Result<Vec<String>> {
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
        return Err(anyhow::Error::msg(format!(
            "Couldn't run FFmpeg for file {}",
            video.path
        )));
    }

    let thumbnails = file_management::list_matching_files(temp_folder, video.id.as_str());
    Ok(thumbnails)
}

async fn run_resume_model_for_video(
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
        let options = GenerationOptions::default().temperature(0.1);
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

async fn run_computer_vision_model_for_video(
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
    let options = GenerationOptions::default().temperature(0.1);
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

fn get_video_metadata(
    video_path: &str,
) -> Result<(Option<f32>, Option<DateTime<Utc>>), Box<dyn std::error::Error>> {
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
        .output()?;

    if !output.status.success() {
        return Err(format!("ffprobe failed to execute: {:?}", output.stderr).into());
    }

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

fn get_capture_gap(duration: f32) -> i32 {
    let f_gap = duration / 3.0;
    f_gap.floor() as i32
}
