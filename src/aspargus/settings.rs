use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::file_management;

/// Represents the Aspargus settings.
///
/// ### Fields
/// - `computer_vision_model`: The name of the computer vision model.
/// - `computer_vision_server`: The server URL for the computer vision model.
/// - `computer_vision_server_port`: The port of server URL for the computer vision model.
/// - `text_model`: The name of the text model.
/// - `text_server`: The server URL for the text model.
/// - `text_server_port`: The port of server URL for the text model.
/// - `work_folder`: The path to the work folder.
/// - `temp_folder`: The path to the temp folder.
/// - `settings_path`: The path to the settings file.
/// - `two_steps`: Flag if the analysis must be performed in two steps or not.
#[derive(Default, Deserialize, Serialize, Debug)]
pub struct AspargusSettings {
    #[serde(default = "get_default_cv_model")]
    pub computer_vision_model: String,
    #[serde(default = "get_default_text_model")]
    pub text_model: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub work_folder: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub temp_folder: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub settings_path: String,
    #[serde(default = "get_default_server_url")]
    pub computer_vision_server: String,
    #[serde(default = "get_default_server_url")]
    pub text_server: String,
    #[serde(default = "get_default_server_port")]
    pub computer_vision_server_port: u16,
    #[serde(default = "get_default_server_port")]
    pub text_server_port: u16,
    #[serde(default = "get_default_two_steps")]
    pub two_steps: bool,
}

/// Gets the default computer vision model.
///
/// ### Returns
/// The default computer vision model.
#[doc(hidden)]
fn get_default_cv_model() -> String {
    "llava-llama3:latest".to_string()
}

/// Gets the default text model.
///
/// ### Returns
/// The default text model.
#[doc(hidden)]
fn get_default_text_model() -> String {
    "mistral".to_string()
}

/// Gets the default server URL.
///
/// ### Returns
/// The default server URL.
#[doc(hidden)]
fn get_default_server_url() -> String {
    "http://localhost".to_string()
}

/// Gets the default server port.
///
/// ### Returns
/// The default server port.
#[doc(hidden)]
fn get_default_server_port() -> u16 {
    11434
}

/// Gets the default two steps flag value.
///
/// ### Returns
/// The default two steps flag value.
#[doc(hidden)]
fn get_default_two_steps() -> bool {
    false
}

/// Loads the Aspargus settings, and creates a new file if it doesn't exist yet.
///
/// ### Returns
/// The Aspargus settings.
pub fn load_settings() -> AspargusSettings {
    let (work_folder, temp_folder) =
        file_management::make_app_folders().expect("Application folders are created");
    let mut settings_path = PathBuf::from(work_folder.clone());
    settings_path.push("settings.json");
    match fs::read_to_string(settings_path.clone()) {
        Ok(settings) => {
            let mut aspargus_settings: AspargusSettings =
                serde_json::from_str(&settings).expect("Could not deserialize settings");
            aspargus_settings.work_folder = work_folder;
            aspargus_settings.temp_folder = temp_folder;
            aspargus_settings.settings_path = settings_path.to_str().unwrap().to_string();
            log::debug!("Loaded settings: {:?}", aspargus_settings);
            aspargus_settings
        }
        Err(_) => {
            log::debug!("No settings file found, creating a new one");
            let aspargus_settings = AspargusSettings {
                computer_vision_model: get_default_cv_model(),
                text_model: get_default_text_model(),
                work_folder: work_folder,
                temp_folder: temp_folder,
                settings_path: settings_path.to_str().unwrap().to_string(),
                computer_vision_server: get_default_server_url(),
                text_server: get_default_server_url(),
                computer_vision_server_port: get_default_server_port(),
                text_server_port: get_default_server_port(),
                two_steps: get_default_two_steps(),
            };
            save_settings(&aspargus_settings).expect("Saving settings file");
            aspargus_settings
        }
    }
}

/// Saves the Aspargus settings to a file.
///
/// ### Parameters
/// - `aspargus_settings`: The Aspargus settings.
///
/// ### Returns
/// An empty Result in case of success.
///
/// ### Errors
/// Returns an error if the export fails.
pub fn save_settings(aspargus_settings: &AspargusSettings) -> anyhow::Result<()> {
    let settings = match serde_json::to_string(aspargus_settings) {
        Ok(settings_serialized) => settings_serialized,
        Err(_) => {
            return Err(anyhow::Error::msg(
                "Error while serializing the Settings file",
            ))
        }
    };
    match fs::write(
        &PathBuf::from(aspargus_settings.settings_path.to_string()),
        settings,
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow::Error::msg(format!(
            "Could not save settings file: {}",
            aspargus_settings.settings_path
        ))),
    }
}
