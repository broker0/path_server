use std;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::sync::Arc;
use std::time::{Duration, Instant};
use log::{trace, debug, info, warn, error, LevelFilter};

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


fn run_service() {

    let start = Instant::now();

    info!("loading data from files, creating the world...");
    let world_model = Arc::new(WorldModel::new(Arc::new(WorldData::new())));
    // patch_world(world_model.clone());
    info!("the creation completed in {:?}", start.elapsed());

    let (http_stop_tx, http_stop_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = {
        let model = world_model.clone();
        std::thread::spawn(move || {
            http::http_server_service(model, http_stop_rx);
        })
    };

    // TODO start condition from config/command line
    if true {
        run_app(world_model);
    }

    info!("app stopped");
    http_stop_tx.send(()).unwrap();
    handle.join().unwrap();
    std::thread::sleep(Duration::from_secs(5));
}


fn initialize_logging() {
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
        .get_matches();


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
    let quiet = matches.get_flag("quiet");
    let term_loglevel = if quiet { LevelFilter::Off } else { loglevel };

    let logfile = match (matches.value_source("logfile"), matches.get_one::<String>("logfile")) {
        (Some(ValueSource::CommandLine), Some(file_name)) => {
            Some(file_name)
        },
        _ => None,
    };

    if let Some(logfile) = logfile {
        CombinedLogger::init(
            vec![
                TermLogger::new(term_loglevel, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
                WriteLogger::new(loglevel, Config::default(), File::create(logfile).unwrap())
            ]
        ).unwrap();
    } else {
        TermLogger::init(term_loglevel, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();
    }
    // println!("{:?} {:?}", loglevel, logfile);
}

fn main() {
    initialize_logging();
    run_service();
}
