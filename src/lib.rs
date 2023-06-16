use std;
use std::collections::{BTreeSet, HashMap};
use std::ffi::{c_char, CStr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::oneshot::Sender;
use lazy_static::lazy_static;
use std::sync::Mutex;
use log::{debug, info, warn};

use crate::world::{WorldModel, WorldTile};

mod mul;
mod http;
mod world;

use mul::*;
use world::world_model::WorldData;


struct ServerControl {
    stop_signal: Sender<()>,
    handle: JoinHandle<()>,
}


lazy_static! {
    static ref SERVER_CONTROL: Mutex<Option<ServerControl>> = Mutex::new(None);
}


#[no_mangle]
pub extern "C" fn start_path_server(data_path: *const c_char, ui_file: *const c_char, http_port: u16) -> bool {
    info!("try start path server");
    {
        let mut control = SERVER_CONTROL.lock().unwrap();
        if control.is_some() {
            warn!("path_server already started");
            return false;
        }

        let mul_path =  unsafe { CStr::from_ptr(data_path) }.to_str().unwrap();
        let ui_path = unsafe { CStr::from_ptr(ui_file) }.to_str().unwrap();

        *control = run_service(Path::new(mul_path), PathBuf::from(ui_path), http_port);
        debug!("path_server started");
    }

    true
}


#[no_mangle]
pub extern "C" fn stop_path_server() -> bool {
    info!("try stop path server");
    {
        let mut control = SERVER_CONTROL.lock().unwrap();
        match control.take() {
            None => {
                warn!("path_server already stopped");
                return false
            },

            Some(control) => {
                control.stop_signal.send(()).unwrap();
                control.handle.join().unwrap();
                debug!("path_server stopped");
            }
        }
    }

    true
}


fn run_service(data_path: &Path, ui_file: PathBuf, http_port: u16) -> Option<ServerControl> {
    let world_model = Arc::new(WorldModel::new(&data_path));
    let (http_stop_tx, http_stop_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = {
        let model = world_model.clone();
        std::thread::spawn(move || {
            http::http_server_service(model, ui_file, http_port, http_stop_rx);
        })
    };

    Some(ServerControl{
        stop_signal: http_stop_tx,
        handle,
    })
}
