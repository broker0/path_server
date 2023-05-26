use std::convert::Infallible;
use std::fs;
use std::io::{Cursor};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio;
use tokio::sync::oneshot::Receiver;

use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};

use serde::{Deserialize, Serialize};

use image::{ImageBuffer, Rgb};
use log::{error, info};

use crate::world::{WorldModel, WorldSurveyor};


#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DistanceFunc {
    Manhattan,
    Chebyshev,
    Diagonal,
    Euclidean,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceOptions {
    // area
    pub left: Option<isize>,
    pub top: Option<isize>,
    pub right: Option<isize>,
    pub bottom: Option<isize>,
    // accuracy
    pub accuracy_x: Option<isize>,
    pub accuracy_y: Option<isize>,
    pub accuracy_z: Option<isize>,
    // misc
    pub all_points: Option<bool>,
    pub open_doors: Option<bool>,
    pub allow_diagonal_move: Option<bool>,
    pub cost_limit: Option<isize>,
    // movement cost
    pub cost_turn: Option<isize>,
    pub cost_move_straight: Option<isize>,
    pub cost_move_diagonal: Option<isize>,
    // heuristic
    pub heuristic_distance: Option<DistanceFunc>,
    pub heuristic_straight: Option<isize>,
    pub heuristic_diagonal: Option<isize>,
}


impl TraceOptions {
    pub fn empty() -> Self {
        Self {
            left: None,
            top: None,
            right: None,
            bottom: None,
            accuracy_x: None,
            accuracy_y: None,
            accuracy_z: None,
            open_doors: None,
            cost_limit: None,
            cost_turn: None,
            cost_move_straight: None,
            cost_move_diagonal: None,
            allow_diagonal_move: None,
            heuristic_distance: None,
            heuristic_straight: None,
            heuristic_diagonal: None,
            all_points: None,
        }
    }


}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    pub world: u8,
    pub serial: u32,
    pub graphic: u32,
    pub x: isize,
    pub y: isize,
    pub z: i8,
    pub timestamp: Option<u64>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Point {
    pub x: isize,
    pub y: isize,
    pub z: i8,
    pub w: isize,
}


#[derive(Serialize, Deserialize, Debug)]
pub enum ApiRequest {
    WorldSave{file_name: String, },
    WorldLoad{file_name: String, },
    WorldClear{},

    ItemsDel {serials: Vec<u32>, },
    ItemsAdd {items: Vec<Item>, },

    Query {world: u8, left: isize, top: isize, right: isize, bottom: isize, },

    TracePath{world: u8, sx: isize, sy: isize, sz: i8, dx: isize, dy: isize, dz: i8, options: TraceOptions, },

    RenderArea{world: u8, left: Option<isize>, top: Option<isize>, right: Option<isize>, bottom: Option<isize>, color: Option<isize>, points: Vec<Point>, },
}


#[derive(Serialize, Deserialize)]
pub enum ApiResponse {
    Success { },
    Error { err: String, },
    QueryReply {items: Vec<Item>, },
    TraceReply { points: Vec<Point>, },
    #[serde(skip_serializing, skip_deserializing)]
    RenderReply { image: ImageBuffer<Rgb<u8>, Vec<u8>> },
}


#[derive(Clone)]
struct ApiHandler {
    world_model: Arc<WorldModel>,
}


impl ApiHandler {
    pub fn new(context: Arc<WorldModel>) -> Self {
        Self {
            world_model: context
        }
    }


    async fn handle_request(&self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        // reading the request body as bytes
        let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|err|{
            let response = ApiResponse::Error {err: format!("Failed to read request body: {}", err) };
            let response_body = serde_json::to_string(&response).unwrap();
            error!("Api::error body parsing - {err}");

            Ok::<Response<Body>, Infallible>(Response::builder()
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap())
        }).unwrap();

        // trying to deserialize it from json to an enum instance `ApiRequest`
        let api_request = serde_json::from_slice::<ApiRequest>(&body_bytes);

        let api_response = match api_request {
            // the request was successfully parsed, we execute it and get a response
            Ok(api_request) => {
                match api_request {
                    ApiRequest::WorldSave{file_name}
                        => self.handle_world_save(&file_name).await,
                    ApiRequest::WorldLoad{file_name}
                        => self.handle_world_load(&file_name).await,
                    ApiRequest::WorldClear{}
                        => self.handle_world_clear(),

                    ApiRequest::ItemsDel {serials}
                        => self.handle_items_del(&serials),
                    ApiRequest::ItemsAdd {items}
                        => self.handle_items_add(&items),

                    ApiRequest::Query {world, left, top, right, bottom}
                        => self.handle_query(world, left, top, right, bottom),

                    ApiRequest::TracePath{world, sx, sy, sz, dx, dy, dz, options}
                        => self.handle_trace_path(world, sx, sy, sz, dx, dy, dz, &options).await,

                    ApiRequest::RenderArea {world, left, top, right, bottom, color, points}
                        => self.handle_render_area(world, left, top, right, bottom, color, &points).await,
                }
            },

            // parsing failed, make a response that will include a description of the error
            Err(err) => {
                error!("Api::error request parsing - {err}");

                ApiResponse::Error {err: format!("Failed to parse request body: {err}") }
            }
        };

        Self::serialize_response(&api_response)
    }


    fn serialize_response(response: &ApiResponse) -> Result<Response<Body>, Infallible> {
        match response {
            // common case, serialize reply to json
            ApiResponse::Success { .. } |
            ApiResponse::Error { .. } |
            ApiResponse::QueryReply { .. } |
            ApiResponse::TraceReply { .. } => {
                let response_body = serde_json::to_string(&response).unwrap();

                Ok(Response::builder()
                    .header("Content-Type", "application/json")
                    .body(response_body.into())
                    .unwrap())
            }

            // special case, RenderReply return png image, not json
            ApiResponse::RenderReply { image } => {
                let mut write_buffer = Cursor::new(Vec::with_capacity(image.width() as usize *image.height() as usize));
                image.write_to(&mut write_buffer, image::ImageOutputFormat::Png).expect("Error while write image to buffer");
                let image_buffer = write_buffer.into_inner();

                Ok(Response::builder()
                       .header("Content-Type", "image/png")
                       .header("Content-Length", image_buffer.len())
                       .body(Body::from(image_buffer))
                       .unwrap())
            }
        }
    }

    // Handlers
    async fn handle_world_save(&self, file_name: &String) -> ApiResponse {
        info!("Api::world_save to {file_name}");
        self.world_model.save_state(file_name);
        ApiResponse::Success {}
    }


    async fn handle_world_load(&self, file_name: &String) -> ApiResponse {
        info!("Api::world_load from {file_name}");
        self.world_model.load_state(file_name);
        ApiResponse::Success {}
    }


    fn handle_world_clear(&self) -> ApiResponse {
        info!("Api::world_clear");
        self.world_model.clear_state();
        ApiResponse::Success {}
    }


    fn handle_items_del(&self, serials: &Vec<u32>) -> ApiResponse {
        info!("Api::item_del {} items", serials.len());
        for serial in serials {
            self.world_model.delete_item(*serial);
        }
        ApiResponse::Success {}
    }


    fn handle_items_add(&self, items: &Vec<Item>) -> ApiResponse {
        info!("Api::item_add {} items", items.len());

        let start = SystemTime::now();
        let since_epoch = start.duration_since(UNIX_EPOCH).expect("Failed to get current time");
        let current_time = since_epoch.as_secs();

        for Item{ world, serial, graphic, x, y, z, .. } in items {
            self.world_model.insert_item(*world, *x, *y, *z, *serial, *graphic, current_time);
        }
        ApiResponse::Success {}
    }


    #[allow(dead_code)]
    fn handle_query(&self, world: u8, left: isize, top: isize, right: isize, bottom: isize) -> ApiResponse {
        info!("Api::query world: {world}, area: {left}, {top} - {right}, {bottom}");
        let mut items = Vec::new();
        self.world_model.query(world, left, top, right, bottom, &mut items);

        ApiResponse::QueryReply { items }
    }


    #[allow(dead_code)]
    async fn handle_trace_path(&self, world: u8, sx: isize, sy: isize, sz: i8, dx: isize, dy: isize, dz: i8, options: &TraceOptions) -> ApiResponse {
        info!("Api::trace_path world {world}, from {sx}, {sy}, {sz} -> to {dx}, {dy}, {dz}");
        let model = self.world_model.clone();

        let options = options.clone();
        let task = tokio::task::spawn_blocking(move || {
            let mut points = Vec::new();
            let world = model.world(world);
            let surv = WorldSurveyor::new(world);
            surv.trace_a_star(sx, sy, sz, 0, dx, dy, dz, 0, &mut points, &options);
            points
        });

        let points = task.await.unwrap();
        ApiResponse::TraceReply { points }
    }


    #[allow(dead_code)]
    async fn handle_render_area(&self, world: u8, left: Option<isize>, top: Option<isize>, right: Option<isize>, bottom: Option<isize>, color: Option<isize>, points: &Vec<Point>) -> ApiResponse {
        // TODO do rendering in a separate thread, as well as path calculation
        let curr_world = self.world_model.world(world);

        let (bound_left, bound_top, bound_right, bound_bottom) = if points.len() > 0 {
            let mut left = isize::MAX;
            let mut top = isize::MAX;
            let mut right = isize::MIN;
            let mut bottom = isize::MIN;

            for point in points {
                left = left.min(point.x);
                top = top.min(point.y);

                right = right.max(point.x);
                bottom = bottom.max(point.y);
            }

            (left-100, top-100, right+100, bottom+100)
        } else {
            (0, 0, curr_world.base.width() as isize, curr_world.base.height() as isize)
        };

        let left = left.unwrap_or(bound_left);
        let top = top.unwrap_or(bound_top);
        let right = right.unwrap_or(bound_right);
        let bottom = bottom.unwrap_or(bound_bottom);
        info!("Api::render_area world {world}, area: {left}, {top} - {right}, {bottom}");

        // if no color is passed to the function, use the default value -1, this value is treated as a special case
        let color = color.unwrap_or(-1);
        let draw_color = Rgb([(color & 0xFF) as u8, ((color >> 8) & 255) as u8, ((color >> 16) & 255) as u8]);

        let (left, right) = (left.min(right), left.max(right));
        let (top, bottom) = (top.min(bottom), top.max(bottom));

        let width = right-left;
        let height = bottom-top;

        info!("Render area: {left},{top}  -  {right},{bottom}");

        let mut tiles = Vec::new();
        let mut image = ImageBuffer::new(width as u32, height as u32);

        // draw map
        for x in left..right {
            for y in top..bottom {
                let px = (x - left) as u32;
                let py = (y - top) as u32;

                tiles.clear();
                curr_world.query_tile_full(x, y, 0, &mut tiles);
                let top_tile = tiles.last().unwrap();
                let color = curr_world.world_tile_color(&top_tile);

                image.put_pixel(px, py, Rgb([color.0, color.1, color.2]));
            }
        }

        // draw points
        for &Point{x,y,z, .. } in points {
            if x < left || x >= right || y < top || y >= bottom {
                continue
            };

            let x = x - left;
            let y = y - top;

            if color == -1 {
                image.put_pixel(x as u32, y as u32, Rgb([0, (z as i16).saturating_add(128) as u8, 0]));
            } else {
                image.put_pixel(x as u32, y as u32, draw_color);
            }
        }


        ApiResponse::RenderReply { image }
    }
}


async fn handle_request(api: Arc<ApiHandler>, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.method() == Method::POST && req.uri().path() == "/api/" {
        let api = api.as_ref();
        return api.handle_request(req).await;
    }

    if req.method() == Method::GET && req.uri().path() == "/ui/" {
        if let Ok(file_contents) = fs::read_to_string("www/ui.html") {
            let response = Response::builder()
                .body(Body::from(file_contents))
                .unwrap();
            return Ok(response);
        }
    }

    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap();

    Ok(response)
}


async fn http_svc(model: Arc<WorldModel>, http_stop: Receiver<()>) {
    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    let api_handler = Arc::new(ApiHandler::new(model));

    let make_service = make_service_fn(move |_conn| {
        let api = api_handler.clone(); // clone the Arc reference
        let service = service_fn(move |req| {
            handle_request(api.clone(), req) // use the Arc reference
        });

        async move { Ok::<_, Infallible>(service) }
    });

    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(async {
            http_stop.await.ok();
        });

    info!("Listening on http://{}", addr);
    if let Err(e) = server.await {
        error!("server error: {}", e);
    } else {
        error!("server stopped successfully")
    }
}

pub fn http_server_service(model: Arc<WorldModel>, http_stop: Receiver<()>) {
    // start http service in single thread runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_name("http_server_thread")
        //.worker_threads(8)    // TODO set worker_threads from config/parameters
        .build()
        .unwrap();

    // block thread while service is running
    rt.block_on(http_svc(model, http_stop));
}
