use std::sync::Arc;
use std::time::{Duration, Instant};
use doryen_rs::{App, AppOptions, Console, DoryenApi, Engine, TextAlign, UpdateEvent};
use crate::mul::tiledata::MulTileFlags;
use crate::world::{WorldModel, WorldSurveyor, WorldTile};


pub struct MulViewer {
    world_model: Arc<WorldModel>,

    is_centered: bool,
    offset_x: isize,
    offset_y: isize,

    current_world: u8,
    current_x: isize,
    current_y: isize,
    current_z: i8,
    current_direction: u8,

    ground_z: i8,
    max_z: i8,

    next_step: Instant,
}

impl MulViewer {
    fn new(world_model: Arc<WorldModel>) -> Self {
        Self {
            world_model,
            is_centered: true,
            offset_x: 0,
            offset_y: 0,
            current_world: 0,
            current_x: 1438,
            current_y: 1697,
            current_z: 0,
            current_direction: 0,
            ground_z: 0,
            max_z: 127,

            next_step: Instant::now(),
        }
    }

    fn control(&mut self, dx: isize, dy: isize) -> (isize, isize){
        let now = Instant::now();
        if self.next_step > now {
            return (0, 0);
        }

        if dx == 0 && dy == 0 {
            return (0, 0);
        }

        let surveyor = WorldSurveyor::new(self.world_model.world(self.current_world));
        let (old_x, old_y) = (self.current_x, self.current_y);

        if dx.abs() <=1 && dy.abs() <= 1 {
            let (x, y, z) = (self.current_x, self.current_y, self.current_z);

            let (z_dest, direction) = {
                let mut direction = WorldSurveyor::direction(dx, dy);
                let mut z_dest = surveyor.test_step(x, y, z, direction);

                if z_dest.is_some() {
                    (z_dest, direction)
                } else if direction & 1 != 0 {
                    direction = WorldSurveyor::direction(0, dy);
                    z_dest = surveyor.test_step(x, y, z, direction);
                    if z_dest.is_none() {
                        direction = WorldSurveyor::direction(dx, 0);
                        z_dest = surveyor.test_step(x, y, z, direction);
                    }
                    (z_dest, direction)
                } else {
                    (z_dest, direction)
                }
            };

            if let Some(new_z) = z_dest {
                // turn
                if self.current_direction != direction {
                    self.next_step = now + Duration::from_millis(50);
                    self.current_direction = direction;
                    return (0, 0)
                }
                // step
                self.next_step = now + Duration::from_millis(100);
                (self.current_x, self.current_y) = WorldSurveyor::move_to(x, y, direction);
                self.current_z = new_z;
            }
        } else {    // fly
            self.current_x += dx;
            self.current_y += dy;
            self.current_z = 111;
            self.next_step = now + Duration::from_millis(100);
        }

        self.current_x = self.current_x.rem_euclid(surveyor.model.base.width() as isize);
        self.current_y = self.current_y.rem_euclid(surveyor.model.base.height() as isize);


        // TODO move to `test_step`?
        self.max_z = 127;
        let world = surveyor.model;
        self.ground_z = world.query_tile_ground(self.current_x, self.current_y, 0).z_base();

        let mut tiles = Vec::with_capacity(16);
        surveyor.get_tile_objects(self.current_x, self.current_y, self.current_direction, &mut tiles);

        for tile in tiles {
            let z_base = tile.z_base();

            if (z_base > self.current_z + 16) && self.max_z > z_base {
                self.max_z = z_base;
            }
        }

        (self.current_x-old_x, self.current_y-old_y)
    }

    fn draw_area(&mut self, x_screen: i32, y_screen: i32, x_world: i32, y_world: i32, area_width: i32, area_height: i32, con: &mut Console) {
        let world = &self.world_model.world(self.current_world);
        let mut tiles = Vec::with_capacity(64);

        for y in y_world..y_world + area_height {
            for x in x_world..x_world + area_width {
                tiles.clear();
                world.query_tile_full(x as isize, y as isize, 0, &mut tiles);


                if tiles.len() != 0 {
                    let mut draw_tile = None::<&WorldTile>;

                    for tile in tiles.iter().rev() {
                        let tile_flag = world.world_tile_flag(tile);

                        // skip tiles over head except land tile
                        if tile.z_base() >= self.max_z && !tile.is_land() {
                            continue
                        }

                        // skip land tiles overhead
                        if self.ground_z > self.current_z && tile.z_base() > self.current_z+16 && tile.is_land() {
                            continue
                        }

                        // no draw roofs
                        if tile_flag & MulTileFlags::Roof as u32 != 0 {
                            continue;
                        }

                        // no draw roofs if there is something over your head
                        if tile_flag & MulTileFlags::Roof as u32 != 0 && self.max_z < 127 {
                            continue;
                        }


                        if let Some(dtile) = draw_tile {
                            // if tile is above that founded tile, then update
                            if tile.z_base() >= dtile.z_base()  {
                                draw_tile = Some(tile);
                            }
                        } else {
                            draw_tile = Some(tile);
                        }

                        if draw_tile.is_some() {
                            break
                        }
                    }

                    if draw_tile.is_none() {
                        con.cell(
                            x_screen + x - x_world, y_screen + y - y_world,
                            Some(32),
                            Some((0,0,0,255)), Some((0,0,0,255)),
                        );
                        continue
                    }

                    let draw_tile = draw_tile.unwrap();

                    let tile_color = world.world_tile_color(draw_tile);
                    let tile_flags = world.world_tile_flag(draw_tile);

                    let impassable = tile_flags & MulTileFlags::Impassable as u32 != 0;
                    let water = tile_flags & MulTileFlags::Wet as u32 != 0;
                    let door = tile_flags & MulTileFlags::Door as u32 != 0;
                    let land = draw_tile.is_land();
                    let slope = draw_tile.is_slope();

                    let (char_draw, fore) = if door {
                        (35, (192, 128, 0, 255))
                    } else if impassable && !water {
                        (7, (255, 0, 64, 255))
                    } else if slope && !land {
                        (30, (0, 127, 0, 255))
                    } else {
                        (32, (0, 0, 0, 255))
                    };

                    con.cell(
                        x_screen + x - x_world, y_screen + y - y_world,
                        Some(char_draw),
                        Some(fore), Some(tile_color),
                    )

                }
            }
        }
    }

    fn draw_tile_slice(&mut self, xc: i32, yc: i32, con: &mut Console) {
        let world = self.world_model.world(self.current_world);

        let mut tiles = Vec::with_capacity(64);
        world.query_tile_full(self.current_x, self.current_y, 0,&mut tiles);

        let mut dy = 0;
        let cnt = tiles.len() as i32;

        for tile in &tiles {
            let tile_num = tile.tile.num();

            let (is_land, z, flags, z_top) = (tile.is_land(), tile.z_base(), world.world_tile_flag(tile), tile.z_top());

            let bcolor = if is_land {
                (64, 64, 64, 255)
            } else {
                (0, 0, 0, 255)
            };

            let fcolor = if flags & MulTileFlags::Impassable as u32 != 0 {
                (255, 0, 0, 255)
            } else {
                (255, 255, 255, 255)
            };
            con.print(xc, yc + cnt - dy - 1, &format!("0x{tile_num:04X} {z:4} -> {z_top:<2} {flags:08X}"), TextAlign::Left, Some(fcolor), Some(bcolor));
            dy += 1;
        }
    }
}


impl Engine for MulViewer {
    fn init(&mut self, _api: &mut dyn DoryenApi) {
    }

    fn update(&mut self, api: &mut dyn DoryenApi) -> Option<UpdateEvent> {
        let (width, height) = {
            let con = api.con();
            (con.get_width() as i32, con.get_height() as i32)
        };

        let input = api.input();
        let mov_scale = if input.key("ShiftLeft") { 18 } else { 1 };

        let (dx, dy) = if input.mouse_button(2) {
            let mouse_pos = input.mouse_pos();
            let (mx, my) = (mouse_pos.0 as i32, mouse_pos.1 as i32);

            let (x, y, z, direction) = (self.current_x as i32, self.current_y as i32, self.current_z, self.current_direction);

            let map_margin: i32 = 1;

            let center_x = (width - 1) / 2;
            let center_y = (height - 1) / 2;

            let (left, top) = (x - center_x + self.offset_x as i32, y - center_y + self.offset_y as i32);

            let lcx = x as i32 - left;
            let lcy = y as i32 - top;

            let (cx, cy) = (map_margin + lcx, map_margin + lcy);

            let (dx, dy) = ((mx-cx) as isize, (my-cy) as isize);


            fn signum(n: isize) -> isize {
                if n > 0 {
                    1
                } else if n < 0 {
                    -1
                } else {
                    0
                }
            }

            fn get_direction(dx: isize, dy: isize) -> (isize, isize) {
                let dx_sign = signum(dx);
                let dy_sign = signum(dy);
                if dx == 0 || dy == 0 {
                    return (signum(dx_sign), signum(dy_sign))
                }
                let slope_dx = dx.abs()*3 / dy.abs();
                let slope_dy = dy.abs()*3 / dx.abs();

                if slope_dx > 7 {
                    (dx_sign, 0)
                } else if slope_dy > 7 {
                    (0, dy_sign)
                } else {
                    (dx_sign, dy_sign)
                }
            }


            if mov_scale == 1 {
                get_direction(dx, dy)
            } else {
                (dx, dy)
            }
        } else {
            let dx = if input.key("ArrowLeft") {
                -1 * mov_scale
            } else if input.key("ArrowRight") {
                1 * mov_scale
            } else {
                0
            };

            let dy = if input.key("ArrowUp") {
                -1 * mov_scale
            } else if input.key("ArrowDown") {
                1 * mov_scale
            } else {
                0
            };

            (dx, dy)
        };

        let (dx, dy) = self.control(dx, dy);
        if input.key("ControlLeft") {
            self.is_centered = false;
            self.offset_x -= dx;
            self.offset_y -= dy;
        } else {
            self.is_centered = true;
            self.offset_x = 0;
            self.offset_y = 0;
        }

        if input.key_pressed("Tab") {
            if self.current_world == 0 {
                self.current_world = 2
            } else {
                self.current_world = 0
            }
        }

        None
    }

    fn render(&mut self, api: &mut dyn DoryenApi) {
        let con = api.con();
        let width = con.get_width() as i32;
        let height = con.get_height() as i32;

        let (x, y, z, direction) = (self.current_x as i32, self.current_y as i32, self.current_z, self.current_direction);

        let map_margin: i32 = 1;

        let center_x = (width - 1) / 2;
        let center_y = (height - 1) / 2;

        let (left, top) = (x - center_x + self.offset_x as i32, y - center_y + self.offset_y as i32);

        let area_width = width - map_margin * 2;
        let area_height = height - map_margin * 2;

        con.clear(Some((0, 0, 0, 255)), None, Some(' ' as u16));

        self.draw_area(
            map_margin, map_margin,
            left, top,
            area_width, area_height,
            con,
        );

        let lcx = x as i32 - left;
        let lcy = y as i32 - top;

        con.cell(map_margin + lcx, map_margin + lcy, Some('@' as u16), Some((0, 192, 192, 255)), None);

        self.draw_tile_slice(map_margin + 2, map_margin + 2, con);

        // let (z_low, z_high, z_dest) = self.movement_check(cursor_pos.0, cursor_pos.1, 120, 0);
        con.print(0, 0,
                  &format!("position: ({} {} {} {}) max_z: {}", x, y, z, direction, self.max_z),
                  TextAlign::Left,
                  Some((255, 255, 255, 255)),
                  None
        );
    }

    fn resize(&mut self, api: &mut dyn DoryenApi) {
        let screen_size = api.get_screen_size();
        api.con().resize(screen_size.0 / 8, screen_size.1 / 8);
    }
}


pub fn run_app(world_model: Arc<WorldModel>) {
    const FONT_SIZE: u32 = 8;
    const WIDTH: u32 = 80;
    const HEIGHT: u32 = 50;

    let mut app = App::new(AppOptions {
        window_title: "Ultima Online".to_owned(),
        // // font_path: "terminal_12x12.png".to_owned(),
        font_path: "terminal_8x8.png".to_owned(),
        console_width: WIDTH,
        console_height: HEIGHT,
        screen_width: WIDTH * FONT_SIZE,
        screen_height: HEIGHT * FONT_SIZE,

        ..Default::default()
    });

    {
        app.set_engine(Box::new(MulViewer::new(world_model)));
        app.run();
    }
}
