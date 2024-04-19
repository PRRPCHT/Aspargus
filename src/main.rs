use clap::{arg, command, value_parser, ArgAction};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
mod aspargus;
use aspargus::file_management;
use aspargus::Aspargus;

#[tokio::main]
async fn main() {
    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    SimpleLogger::new()
        .with_colors(true)
        .with_level(level)
        .with_module_level("ollama_rs", LevelFilter::Info)
        .init()
        .unwrap();

    let mut aspargus = Aspargus::new();
    let matches = command!() // requires `cargo` feature
        .arg(
            arg!([videos] "Optional videos paths to analyse")
                .action(ArgAction::Append)
                .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -f --folder <PATH> "The folder where the videos are situated"
            )
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(
                -s --start <FILE> "The name of the first file to analyse (alphabetically)"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -e --end <FILE> "The name of the last file to analyse (alphabetically)"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -r --rename <TEMPLATE> "The template of the new file name"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -j --json <PATH> "The path of the JSON file to export the analysis result"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -c --cv_model <NAME> "The name of the computer vision model to use"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                -t --text_model <NAME> "The name of the text model to use"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .get_matches();

    let folder = if let Some(folder_path) = matches.get_one::<PathBuf>("folder") {
        log::debug!("Folder to analyse: {}", folder_path.display());
        Some(folder_path)
    } else {
        None
    };

    let start_file = if let Some(start) = matches.get_one::<String>("start") {
        log::debug!("Start file: {}", start);
        Some(start.as_str())
    } else {
        None
    };

    let end_file = if let Some(end) = matches.get_one::<String>("end") {
        log::debug!("End file: {}", end);
        Some(end.as_str())
    } else {
        None
    };

    let rename_template = if let Some(rename_template) = matches.get_one::<String>("rename") {
        log::debug!("Renaming template: {}", rename_template);
        Some(rename_template.as_str())
    } else {
        None
    };

    let json_path = if let Some(json_path) = matches.get_one::<String>("json") {
        log::debug!("JSON file path: {}", json_path);
        Some(json_path.as_str())
    } else {
        None
    };

    if let Some(cv_model) = matches.get_one::<String>("cv_model") {
        log::debug!("Computer Vision model: {}", cv_model);
        aspargus.set_computer_vision_model(cv_model.to_string());
    };

    if let Some(text_model) = matches.get_one::<String>("text_model") {
        log::debug!("Text model: {}", text_model);
        aspargus.set_text_model(text_model.to_string());
    };

    let files = if let Some(files) = matches.get_many::<String>("videos") {
        if start_file.is_some() || end_file.is_some() || folder.is_some() {
            log::warn!("When a list of video files is given as argument, folder, start and end are not taken in account");
        }
        let the_files = files.map(|v| v.to_string()).collect::<Vec<_>>();
        log::debug!("Value for name: {:?}", the_files);
        Some(the_files)
    } else if let Some(folder) = folder {
        Some(file_management::filter_files_in_dir(
            folder, start_file, end_file,
        ))
    } else {
        None
    };

    if (start_file.is_some() || end_file.is_some()) && folder.is_none() && files.is_none() {
        log::error!(
            "When using the start or end arguments, the folder argument must not be empty."
        );
        return;
    }

    aspargus.add_videos(files.unwrap_or_default());
    aspargus.extract_frames();
    aspargus.run_computer_vision_model().await;
    aspargus.run_resume_model().await;

    if rename_template.is_some() {
        match aspargus.rename_videos(rename_template.unwrap()) {
            Ok(_) => (),
            Err(error) => log::error!("Error while renaming the videos: {}", error),
        };
    }

    if json_path.is_some() {
        match aspargus.export_to_json(json_path.unwrap()) {
            Ok(_) => (),
            Err(error) => log::error!("Error while exporting the JSON file: {}", error),
        };
    }
}
