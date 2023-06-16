use std;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant};
use log::{error, info, LevelFilter};

use crate::world::{WorldModel, WorldTile};

use mul::*;
use world::world_model::WorldData;
use crate::ui::viewer::run_app;

use clap;
use clap::{arg, ArgAction, command};
use clap::parser::ValueSource;
use simplelog::{ColorChoice, CombinedLogger, TerminalMode, TermLogger, WriteLogger, Config};

mod ui;
mod mul;
mod http;
mod world;


fn run_service(data_path: &Path, ui_file: PathBuf, http_port: u16) {
    let start = Instant::now();

    info!("loading data from files, creating the world...");
    let world_model = Arc::new(WorldModel::new(&data_path));
    info!("the creation completed in {:?}", start.elapsed());

    let (http_stop_tx, http_stop_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = {
        let model = world_model.clone();
        std::thread::spawn(move || {
            http::http_server_service(model, ui_file, http_port, http_stop_rx);
        })
    };

    // TODO start condition from config/command line
    if true {
        run_app(world_model);
    }

    info!("app stopped");
    http_stop_tx.send(()).unwrap();
    handle.join().unwrap();
    info!("server stopped");
}


fn initialize_logging(loglevel: LevelFilter, quiet: bool, logfile: Option<&String>) {
    let term_loglevel = if quiet { LevelFilter::Off } else { loglevel };

    if let Some(logfile) = logfile {
        CombinedLogger::init(
            vec![
                TermLogger::new(term_loglevel, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
                WriteLogger::new(loglevel, Config::default(), File::create(logfile).unwrap())
            ]
        ).unwrap();
    } else {
        TermLogger::init(term_loglevel, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();
    };
}


fn parse_cmd_args() -> (PathBuf, PathBuf, u16) {
    let matches = command!()
        .next_line_help(true)
        .arg(
            arg!(--loglevel <LEVEL>)
                .required(false)
                .default_value("debug")
                .value_parser(["trace", "debug", "info", "warn", "error", "off" ])
                .action(ArgAction::Set)
        )
        .arg(
            arg!(--logfile [FILE_NAME])
                .required(false)
                .default_value("path_server.log")
                .action(ArgAction::Set)
                .help("Enables logging to a file. Disabled by default")
        )
        .arg(
            arg!(-q --quiet)
                .required(false)
                .action(ArgAction::SetTrue)
                .help("Disables output to the terminal")
        )
        .arg(
            arg!(--data)
                .required(false)
                .default_value(".")
                .action(ArgAction::Set)
                .help("Specifies the directory with Ultima Online data files.")
        )
        .arg(
            arg!(--ui)
                .required(false)
                .default_value("www/ui.html")
                .action(ArgAction::Set)
                .help("Sets the filename with web-ui.")
        )
        .arg(
            arg!(-p --port)
                .required(false)
                .default_value("3000")
                .action(ArgAction::Set)
                .help("Sets the http server port.")
        )
        .get_matches();


    let quiet = matches.get_flag("quiet");

    let loglevel = match matches.get_one::<String>("loglevel") {
        None => LevelFilter::Off,
        Some(level) => {
            match level.as_str() {
                "trace" => LevelFilter::Trace,
                "debug" => LevelFilter::Debug,
                "info" => LevelFilter::Info,
                "warn" => LevelFilter::Warn,
                "error" => LevelFilter::Error,
                "off" => LevelFilter::Off,
                _ => unreachable!(),
            }
        }
    };

    let logfile = match (matches.value_source("logfile"), matches.get_one::<String>("logfile")) {
        (Some(ValueSource::CommandLine), Some(file_name)) => {
            Some(file_name)
        },
        _ => None,
    };

    initialize_logging(loglevel, quiet, logfile);

    let port = match matches.get_one::<String>("port").unwrap().parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            error!("Error parsing port argument, default value of 3000 will be used.");
            3000
        }
    };

    let mul_dir = PathBuf::from(matches.get_one::<String>("data").unwrap());
    let ui_file = PathBuf::from(matches.get_one::<String>("ui").unwrap());

    (mul_dir, ui_file, port)
}

fn main() {
    let (data_path, ui_file, http_port) = parse_cmd_args();
    run_service(&data_path, ui_file, http_port);
}
