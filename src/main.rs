use clap::parser::ValuesRef;
use clap::ArgMatches;
use clap::{arg, command, value_parser, ArgAction, Command};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::path::PathBuf;
mod aspargus;
use aspargus::file_management;
use aspargus::Aspargus;

/// Builds the args parsing.
///
/// ### Returns
/// The args to be parsed.
fn make_args() -> Command {
    command!() // requires `cargo` feature
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
        .arg(
            arg!(
                --text_server <URL> "The url of the text server to use"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                --text_server_port <PORT> "The port of the text server to use"
            )
            .required(false)
            .value_parser(value_parser!(u16)),
        )
        .arg(
            arg!(
                 --cv_server <URL> "The url and port of the computer vision server to use"
            )
            .required(false)
            .value_parser(value_parser!(String)),
        )
        .arg(
            arg!(
                --cv_server_port <PORT> "The port of the computer vision server to use"
            )
            .required(false)
            .value_parser(value_parser!(u16)),
        )
        .arg(
            arg!(
                 --two_steps "Runs the analysis in two steps, first running the CV model and then running text model to generate a resume"
            )
            .required(false)
            .action(ArgAction::SetTrue), 
        )
}

/// Gets the videos list argument.
///
/// ### Return
/// An Option with a list of videos paths.
fn get_videos(matches: &ArgMatches) -> Option<ValuesRef<String>> {
    matches.get_many::<String>("videos")
}

/// Gets the folder path.
///
/// ### Return
/// An Option with the folder path.
fn get_folder(matches: &ArgMatches) -> Option<&PathBuf> {
    if let Some(folder_path) = matches.get_one::<PathBuf>("folder") {
        log::debug!("Folder to analyse: {}", folder_path.display());
        Some(folder_path)
    } else {
        None
    }
}

/// Gets the start file argument.
///
/// ### Return
/// An Option with the file name of the first video to start with.
fn get_start_file(matches: &ArgMatches) -> Option<&str> {
    if let Some(start) = matches.get_one::<String>("start") {
        log::debug!("Start file: {}", start);
        Some(start.as_str())
    } else {
        None
    }
}

/// Gets the end file argument.
///
/// ### Return
/// An Option with the file name of the last video to end with.
fn get_end_file(matches: &ArgMatches) -> Option<&str> {
    if let Some(end) = matches.get_one::<String>("end") {
        log::debug!("End file: {}", end);
        Some(end.as_str())
    } else {
        None
    }
}

/// Gets the template for the file renaming.
///
/// ### Return
/// An Option with the template for the file renaming.
fn get_rename_template(matches: &ArgMatches) -> Option<&str> {
    if let Some(rename_template) = matches.get_one::<String>("rename") {
        log::debug!("Renaming template: {}", rename_template);
        Some(rename_template.as_str())
    } else {
        None
    }
}

/// Gets the path of the json file to export the results to.
///
/// ### Return
/// An Option with the path of the json file to export the results to.
fn get_json_path(matches: &ArgMatches) -> Option<&str> {
    if let Some(json_path) = matches.get_one::<String>("json") {
        log::debug!("JSON file path: {}", json_path);
        Some(json_path.as_str())
    } else {
        None
    }
}

/// Sets the URL of the computer vision server.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_computer_vision_server(aspargus: &mut Aspargus, matches: &ArgMatches) {
    if let Some(cv_server) = matches.get_one::<String>("cv_server") {
        log::debug!("Computer Vision server: {}", cv_server);
        aspargus.set_computer_vision_server(cv_server.to_string());
    };
}

/// Sets the port of the computer vision server.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_computer_vision_server_port(aspargus: &mut Aspargus, matches: &ArgMatches) {
    if let Some(cv_server_port) = matches.get_one::<u16>("cv_server_port") {
        log::debug!("Computer Vision server port: {}", cv_server_port);
        aspargus.set_computer_vision_server_port(cv_server_port.to_owned());
    };
}

/// Sets the URL of the text server.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_text_server(aspargus: &mut Aspargus, matches: &ArgMatches) {
    if let Some(text_server) = matches.get_one::<String>("text_server") {
        log::debug!("Text server: {}", text_server);
        aspargus.set_text_server(text_server.to_string());
    };
}

/// Sets the port of the text server.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_text_server_port(aspargus: &mut Aspargus, matches: &ArgMatches) {
    if let Some(text_server_port) = matches.get_one::<u16>("text_server_port") {
        log::debug!("Text server port: {}", text_server_port);
        aspargus.set_text_server_port(text_server_port.to_owned());
    };
}

/// Sets the name of the computer vision model.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_computer_vision_model(aspargus: &mut Aspargus, matches: &ArgMatches) {
    if let Some(cv_model) = matches.get_one::<String>("cv_model") {
        log::debug!("Computer Vision model: {}", cv_model);
        aspargus.set_computer_vision_model(cv_model.to_string());
    };
}

/// Sets the name of the text model.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_text_model(aspargus: &mut Aspargus, matches: &ArgMatches) {
    if let Some(text_model) = matches.get_one::<String>("text_model") {
        log::debug!("Text model: {}", text_model);
        aspargus.set_text_model(text_model.to_string());
    };
}

/// Sets the two steps approach flag.
///
/// ### Parameters
/// - `aspargus`: The Aspargus instance.    
/// - `matches`: The app's arguments.
fn set_two_steps(aspargus: &mut Aspargus, matches: &ArgMatches) {
    let two_steps = matches.get_flag("two_steps");
    log::debug!("Two steps analysis: {}", two_steps);
    aspargus.set_two_steps(two_steps);
}

/// Gets the list of video files based on the provided arguments.
///
/// ### Parameters
/// - `videos`: The list of video files to analyse (overrides the 'folder' parameter).    
/// - `folder`: The path of the folder to analyse.
/// - `start_file`: The name of the first file to analyse in the folder.
/// - `end_file`: The name of the last file to analyse in the folder.
fn get_videos_list(
    videos: Option<ValuesRef<String>>,
    folder: Option<&PathBuf>,
    start_file: Option<&str>,
    end_file: Option<&str>,
) -> Option<Vec<String>> {
    if let Some(files) = videos {
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
    }
}

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
    let matches = make_args().get_matches();
    let videos = get_videos(&matches);
    let folder = get_folder(&matches);
    let start_file = get_start_file(&matches);
    let end_file = get_end_file(&matches);
    let rename_template = get_rename_template(&matches);
    let json_path = get_json_path(&matches);
    set_computer_vision_server(&mut aspargus, &matches);
    set_computer_vision_server_port(&mut aspargus, &matches);
    set_computer_vision_model(&mut aspargus, &matches);
    set_text_server(&mut aspargus, &matches);
    set_text_server_port(&mut aspargus, &matches);
    set_text_model(&mut aspargus, &matches);
    set_two_steps(&mut aspargus, &matches);

    let files = get_videos_list(videos, folder, start_file, end_file);
    if (start_file.is_some() || end_file.is_some()) && folder.is_none() && files.is_none() {
        log::error!(
            "When using the start or end arguments, the folder argument must not be empty."
        );
        return;
    }

    if files.is_none() {
        log::error!("No video files to analyse, we're quitting.");
        return;
    }

    aspargus.add_videos(files.unwrap_or_default());
    aspargus.extract_frames();
    if aspargus.is_two_steps() {
        aspargus.run_computer_vision_model().await;
        aspargus.run_resume_model().await;
    } else {
        aspargus.run_only_computer_vision_model().await;
    }

    if rename_template.is_some() {
        aspargus.rename_videos(rename_template.unwrap());
    }

    if json_path.is_some() {
        match aspargus.export_to_json(json_path.unwrap()) {
            Ok(_) => (),
            Err(error) => log::error!("Error while exporting the JSON file: {}", error),
        };
    }
}
