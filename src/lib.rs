use std;
use std::collections::{BTreeSet, HashMap};
use std::ffi::{c_char, CStr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use lazy_static::lazy_static;
use std::sync::Mutex;
use log::{debug, info, warn};

use crate::world::{WorldModel, WorldTile};

mod mul;
mod http;
mod world;

use mul::*;
use world::world_model::WorldData;
use http::server::ServerControl;


lazy_static! {
    static ref SERVER_CONTROL: Mutex<Option<ServerControl>> = Mutex::new(None);
}


fn _start_path_server(data_path: &Path, ui_file: PathBuf, http_port: u16) -> bool {
    info!("try start path server");
    {
        let mut control = SERVER_CONTROL.lock().unwrap();
        if control.is_some() {
            warn!("path_server already started");
            return false;
        }

        let world_model = Arc::new(WorldModel::new(data_path));

        *control = http::server::run_service(world_model, ui_file, http_port);
        debug!("path_server started");
    }

    true
}


#[no_mangle]
pub extern "C" fn start_path_server() -> bool {
    _start_path_server(&Path::new("."), PathBuf::from("www/ui.html"), 3000)
}


#[no_mangle]
pub extern "C" fn start_path_server_ex(data_path: *const c_char, ui_file: *const c_char, http_port: u16) -> bool {
    let data_path =  unsafe { CStr::from_ptr(data_path) }.to_str().unwrap();
    let ui_file = unsafe { CStr::from_ptr(ui_file) }.to_str().unwrap();

    _start_path_server(&Path::new(data_path), PathBuf::from(ui_file), http_port)
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
