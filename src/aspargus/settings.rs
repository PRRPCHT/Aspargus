use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::file_management;

#[derive(Default, Deserialize, Serialize, Debug)]
pub struct AspargusSettings {
    pub computer_vision_model: String,
    pub text_model: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub work_folder: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub temp_folder: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub settings_path: String,
}

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
                computer_vision_model: "llava".to_string(),
                text_model: "mistral".to_string(),
                work_folder: work_folder,
                temp_folder: temp_folder,
                settings_path: settings_path.to_str().unwrap().to_string(),
            };
            save_settings(&aspargus_settings).expect("Saving settings file");
            aspargus_settings
        }
    }
}

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
