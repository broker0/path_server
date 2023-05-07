use std::cmp::{Ordering};
use std::collections::{BinaryHeap, BTreeSet, HashMap, HashSet, VecDeque};
use std::collections::hash_map::{Entry};
use std::time::Instant;

use crate::http::server::{DistanceFunc, Point, TraceOptions};
use crate::world::quadtree::QuadTree;
use crate::world::{DynamicWorld, TileShape, WorldTile};


#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
struct Position (isize, isize, i8);


// fval, gval, dir, dst, src
struct ScoredPosition (isize, isize, u8, Position, Position);

impl PartialEq for ScoredPosition {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ScoredPosition {
}

impl Ord for ScoredPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}


impl PartialOrd for ScoredPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct WorldSurveyor<'a> {
    pub model: &'a DynamicWorld,
}


impl<'a> WorldSurveyor<'a> {
    /// returns a new object
    pub fn new(model: &'a DynamicWorld) -> Self {
        Self {
            model,
        }
    }

    /// returns a vector of elements located at the given coordinates and used in movement testing
    pub fn get_tile_objects(&self, x: isize, y: isize, direction: u8, result: &mut Vec<WorldTile>) {
        self.model.query_tile_full(x, y, direction, result);
    }

    fn get_top_cover(&self, x: isize, y: isize, z: i8) {
        let mut tiles = Vec::with_capacity(16);
        self.get_tile_objects(x, y, 0, &mut tiles);
        // let mut max_z = 127;
    }

    /// returns the lower and upper points of the height range reachable from the current position
    pub fn get_source_step_range(&self, x: isize, y: isize, z: i8, exit_direction: u8) -> (i8, i8) {
        let mut tiles = Vec::with_capacity(16);
        self.get_tile_objects(x, y, exit_direction, &mut tiles);
        let mut z_low_fall = i8::MIN; // lowest available point
        let mut z_high = z;     // highest available point

        for tile in &tiles {
            let (z_base, z_stand, z_top, slope) = match tile.shape {
                TileShape::Surface { z_base, z_stand, .. } =>  (z_base, z_stand, z_stand, false),
                TileShape::Slope { z_base, z_stand, z_top, .. } => (z_base, z_stand, z_top, true),
                TileShape::Background { .. } => continue,
            };

            // if the surface is lower than our position and higher than
            // the lowest point found, then update the lowest point
            if z_stand <= z && z_stand > z_low_fall {
                z_low_fall = z_stand;
            }

            // if we stand exactly on the stairs, then we try to expand
            // the available range by the boundaries of the stairs
            if slope && z_stand == z {
                z_low_fall = z_low_fall.min(z_base);
                z_high = z_high.max(z_top);
            }
        }

        const CLIMB_HEIGHT: i8 = 2;
        (z_low_fall, z_high + CLIMB_HEIGHT)
    }


    /// returns the new z position at the specified point
    /// if there is no matching position, returns None
    pub fn get_dest_position(&self, x: isize, y: isize, z: i8, z_low: i8, z_high: i8) -> Option<i8> {
        let mut objects = Vec::with_capacity(16);
        self.get_tile_objects(x, y, 0, &mut objects);    // direction doesn't matter now.
        objects.push(WorldTile::cap_tile());

        let z = if z < z_low { z_low } else { z };
        let z_high = z_high as i16;
        let mut z_low = z_low as i16;           // the lowest point where we can get on the current iteration
        let mut current_z = i8::MIN as i16;     // the highest point we have seen before current iteration

        let mut result = None::<i8>;

        for (i, upper_obj) in objects.iter().enumerate() {
            let (upper_obj_z_base, upper_obj_z_stand) = match upper_obj.shape {
                TileShape::Slope { z_base, z_stand, .. } => (z_base as i16, z_stand as i16),
                TileShape::Surface { z_base, z_stand, .. } => (z_base as i16, z_stand as i16),
                TileShape::Background { .. } => continue,
            };

            // if z_low > z_high {  // is this correct?
            //     break
            // }

            const CHARACTER_HEIGHT: i16 = 16;
            // character can fit between upper_obj_z_base and z_low
            if upper_obj_z_base - z_low >= CHARACTER_HEIGHT {
                // check the tiles below in reverse order
                for bottom_obj in objects[..i].iter().rev() {
                    let (bottom_obj_z_stand, passable) = match bottom_obj.shape {
                        TileShape::Slope { z_stand, passable,.. }   => (z_stand as i16, passable),
                        TileShape::Surface { z_stand, passable, .. } => (z_stand as i16, passable),
                        TileShape::Background { .. } => continue,
                    };

                    // if bottom_obj_z_stand < z_low {
                    //     break
                    // }

                    // if the tile is walkable, it is higher than the last viewed "upper" tile and
                    // there is enough room for the character to stand between it and the upper_tile_z_base
                    if passable && bottom_obj_z_stand >= current_z && (upper_obj_z_base - bottom_obj_z_stand) >= CHARACTER_HEIGHT {
                        // check if we can reach it from our z_high, given the type of the object
                        if !match bottom_obj.shape {
                            TileShape::Slope { z_base, .. }   => z_base as i16 <= z_high,
                            TileShape::Surface { z_stand, .. } => z_stand as i16 <= z_high,
                            TileShape::Background { .. } => unreachable!(),
                        } {
                            continue
                        };


                        // if the found position is better, remember it
                        if let Some(best_z) = result {
                            let curr_delta = (z_low - bottom_obj_z_stand).abs();
                            let prev_delta = (z - best_z).abs() as i16;

                            if curr_delta < prev_delta {
                                result = Some(bottom_obj_z_stand as i8);
                            }
                        } else {
                            result = Some(bottom_obj_z_stand as i8);
                        }
                    }
                }
            }

			// update variables if necessary
            z_low = z_low.max(upper_obj_z_stand);
			current_z = current_z.max(upper_obj_z_stand);
        }

        result
    }

    #[inline]
    pub fn direction(dx: isize, dy: isize) -> u8 {
        fn signum(n: isize) -> i32 {
            if n > 0 {
                1
            } else if n < 0 {
                -1
            } else {
                0
            }
        }
        match (signum(dx), signum(dy)) {
            (0, -1) => 0u8,
            (1, -1) => 1,
            (1, 0) => 2,
            (1, 1) => 3,
            (0, 1) => 4,
            (-1, 1) => 5,
            (-1, 0) => 6,
            (-1, -1) => 7,
            _ => unreachable!(),
        }
    }

    /// shifts the given coordinates in the specified direction
    #[inline]
    pub fn move_to(x: isize, y: isize, direction: u8) -> (isize, isize) {
        match direction & 7 {
            0 => (x+0, y-1),
            1 => (x+1, y-1),
            2 => (x+1, y+0),
            3 => (x+1, y+1),
            4 => (x+0, y+1),
            5 => (x-1, y+1),
            6 => (x-1, y+0),
            7 => (x-1, y-1),
            _ => unreachable!(),
        }
    }

    /// turns the direction by the specified number of steps, can turn both to the right,
    /// with positive steps, and to the left, with negative steps.
    /// If steps is null, returns the original value
    #[inline]
    fn turn_to(direction: u8, steps: i8) -> u8 {
        (direction as i8 + steps).rem_euclid(8) as u8
    }

    /// just checks if it is possible to step from the starting position in the specified direction
    fn test_step_single(&self, x: isize, y: isize, z: i8, direction: u8) -> Option<i8> {
        let (to_x, to_y) = Self::move_to(x, y, direction);
        let (z_low, z_high) = self.get_source_step_range(x, y, z, direction);
        self.get_dest_position(to_x, to_y, z, z_low, z_high)
    }

    /// a more complete check of the movement, additionally checking
    /// the passability of adjacent tiles for the diagonal direction of the step
    pub fn test_step(&self, x: isize, y: isize, z: i8, direction: u8) -> Option<i8> {
        // check destination tile
        let dest_z = self.test_step_single(x, y, z, direction)?;

        // if not diagonal direction just return result
        if direction & 1 == 0  {
            return Some(dest_z)
        }

        // check adjacent tiles for diagonal step
        // right tile
        self.test_step_single(x, y, z, Self::turn_to(direction, 1))?;

        // left tile
        self.test_step_single(x, y, z, Self::turn_to(direction, -1))?;

        // adjacent tiles ok, return destination z
        Some(dest_z)
    }


    /// searches for a path by algorithm A* from the point s_x,s_y,s_z to the point d_x, d_y, d_z.
    /// `points` will contain the found path to the nearest possible point, or all points explored during the search,
    /// depending on the options.
    /// Also, through `options`, you can fine-tune the parameters of the algorithm, such as the distance function,
    /// heuristic coefficients, boundaries of the path search area.
    pub fn trace_a_star(&self, s_x: isize, s_y: isize, s_z: i8, sdir: u8, d_x: isize, d_y: isize, d_z: i8, ddir: u8, points: &mut Vec<Point>, options: &TraceOptions) {
        let mut cached_steps = HashMap::new();
        let mut frontier = BinaryHeap::new();
        let mut visited = HashMap::new();
        let mut back_path = HashMap::new();

        let x_accuracy = options.accuracy_x.unwrap_or(0);
        let y_accuracy = options.accuracy_y.unwrap_or(0);
        let z_accuracy = options.accuracy_z.unwrap_or(0);

        let cost_limit = options.cost_limit.unwrap_or(isize::MAX);
        let cost_turn = options.cost_turn.unwrap_or(1);
        let cost_move_straight = options.cost_move_straight.unwrap_or(1);
        let cost_move_diagonal = options.cost_move_diagonal.unwrap_or(cost_move_straight);
        let allow_diagonal_move = options.allow_diagonal_move.unwrap_or(false);

        let h_dist = options.heuristic_distance.unwrap_or(DistanceFunc::Diagonal);
        let h_direct = options.heuristic_straight.unwrap_or(5);
        let h_diagonal = options.heuristic_diagonal.unwrap_or(h_direct);

        let left = options.left.unwrap_or(0);
        let top = options.top.unwrap_or(0);
        let right = options.right.unwrap_or(self.model.base.width() as isize);
        let bottom = options.bottom.unwrap_or(self.model.base.height() as isize);

        let all_points = options.all_points.unwrap_or(false);

        let dist_func = |dx: isize, dy: isize| {
            match h_dist {
                DistanceFunc::Manhattan => (dx + dy) * h_direct,
                DistanceFunc::Chebyshev => dx.max(dy) * h_direct,
                DistanceFunc::Diagonal => h_direct * (dx + dy) + (h_diagonal - 2 * h_direct) * dx.min(dy),
                DistanceFunc::Euclidean => f64::sqrt((dx * dx + dy * dy) as f64) as isize * h_direct,
            }
        };

        let h_func = |position: &Position| {
            let dx = (d_x -position.0).abs();
            let dy = (d_y -position.1).abs();
            dist_func(dx, dy)
        };


        let check_step = |x: isize, y: isize, z: i8, dir: u8, cache: &mut HashMap<(isize, isize, i8, u8), Option<i8>>| {
            let (dx, dy) = Self::move_to(x, y, dir);

            if dx < left || dx >= right || dy < top || dy >= bottom { // check bounds
                return None
            }

            match cache.entry((x, y, z, dir)) {
                Entry::Occupied(entry) => {
                    *entry.get()
                }

                Entry::Vacant(entry) => {
                    let result = self.test_step_single(x,y,z, dir);

                    entry.insert(result);
                    result
                }
            }
        };

        let start_pos = Position(s_x, s_y, s_z);
        let start_gval = 0;
        let start_fval = start_gval + h_func(&start_pos);
        let scored_start_pos = ScoredPosition(start_fval, start_gval, sdir, start_pos, Position(-1, -1, -1));
        frontier.push(scored_start_pos);

        let start_time = Instant::now();
        let mut cnt = 0;

        let mut best_dist = isize::MAX;
        let mut best_pos = None;

        while let Some(curr_scored_pos) = frontier.pop() {
            let ScoredPosition(curr_fval, curr_gval, curr_dir, curr_pos, src_pos) = curr_scored_pos;
            let Position(curr_x, curr_y, curr_z) = curr_pos;

            cnt += 1;
            if cnt % 100000 == 0 {
                println!("{cnt} ({curr_x} {curr_y}) current score {curr_fval}, frontier len {}", frontier.len());
            }

            // check if the current position has been visited before
            match visited.entry(curr_pos) {
                Entry::Occupied(_) => { // already visited
                        continue;
                }
                Entry::Vacant(entry) => {   // not visited
                    entry.insert(curr_gval);
                }
            }

            back_path.insert(curr_pos, src_pos);

            // goal check
            let d_x = (d_x - curr_x).abs();
            let d_y = (d_y - curr_y).abs();
            let d_z = (d_z - curr_z).abs() as isize;
            let d_max = d_x.max(d_y).max(d_z);

            if d_max < best_dist {
                best_pos = Some(curr_pos);
                best_dist = d_max;
            }

            if d_x <= x_accuracy && d_y <= y_accuracy && d_z <= z_accuracy {
                println!("Found! {curr_x} {curr_y} {curr_gval} {curr_fval}");
                break
            }

            let dest_n = check_step(curr_x, curr_y, curr_z, 0, &mut cached_steps);
            let dest_e = check_step(curr_x, curr_y, curr_z, 2, &mut cached_steps);
            let dest_s = check_step(curr_x, curr_y, curr_z, 4, &mut cached_steps);
            let dest_w = check_step(curr_x, curr_y, curr_z, 6, &mut cached_steps);

            let steps = if allow_diagonal_move {
                let dest_ne = if dest_n.is_some() && dest_e.is_some() { check_step(curr_x, curr_y, curr_z, 1, &mut cached_steps) } else { None };
                let dest_se = if dest_s.is_some() && dest_e.is_some() { check_step(curr_x, curr_y, curr_z, 3, &mut cached_steps) } else { None };
                let dest_sw = if dest_s.is_some() && dest_w.is_some() { check_step(curr_x, curr_y, curr_z, 5, &mut cached_steps) } else { None };
                let dest_nw = if dest_n.is_some() && dest_w.is_some() { check_step(curr_x, curr_y, curr_z, 7, &mut cached_steps) } else { None };

                [(0, dest_n), (1, dest_ne), (2, dest_e), (3, dest_se), (4, dest_s), (5, dest_sw), (6, dest_w), (7, dest_nw)]
            } else {
                [(0u8, dest_n),  (2, dest_e), (4, dest_s), (6, dest_w), (1, None), (3, None), (5, None), (7, None)]
            };

            for (direction, dest_result) in steps {
                if let Some(dest_z) = dest_result {
                    let (dest_x, dest_y) = Self::move_to(curr_x, curr_y, direction);
                    let dest_pos = Position(dest_x, dest_y, dest_z);

                    match visited.entry(dest_pos) {
                        Entry::Occupied(entry) => {
                            continue;
                        }
                        Entry::Vacant(_) => {}
                    }

                    let dest_gval = curr_gval + if direction & 1 != 0 {
                        if direction == curr_dir { cost_move_diagonal } else { cost_move_diagonal + cost_turn }
                    } else {
                        if direction == curr_dir { cost_move_straight } else { cost_move_straight + cost_turn }
                    };

                    if dest_gval > cost_limit {
                        continue
                    }

                    let dest_fval = dest_gval + h_func(&dest_pos);
                    let dest_scored = ScoredPosition(dest_fval, dest_gval, direction, dest_pos, curr_pos);

                    frontier.push(dest_scored);
                }
            }
        }

        let duration = start_time.elapsed();
        println!("Total tiles explored {cnt} and visited {} in {:?}", visited.len(), duration);

        if all_points {
            for (Position(x, y,z), w) in visited {
                points.push(Point{ x, y, z, w, });
            }
        } else if let Some(mut curr_pos) = best_pos {
            cnt = 0;
            println!("search path to start from {curr_pos:?} with score {best_dist}");
            loop {
                cnt += 1;
                // println!("{cnt} - {curr_pos:?}");
                let prev_pos = back_path[&curr_pos];
                points.push(Point{ x: prev_pos.0, y: prev_pos.1, z: prev_pos.2, w: 0, });

                if prev_pos == start_pos {
                    println!("found start, path len is {cnt} tiles!");
                    break;
                }
                curr_pos = prev_pos;
            }
            points.reverse();
        } else {
            println!("there is no data to return after tracing completes")
        }
    }


    // WIP

    pub fn path_len(&self, x: isize, y: isize, z: i8, dx: isize, dy: isize, dz: i8, max_steps: usize) -> Option<usize> {
        let mut queue = VecDeque::new();
        let mut visited = BTreeSet::new();
        let start_pos = (x, y, z);

        queue.push_front((start_pos, 0));
        while queue.len() > 0 {
            let (curr_pos, cur_gen) = queue.pop_back().unwrap();

            if visited.contains(&curr_pos) {
                continue;
            }
            let (curr_x, curr_y, curr_z) = curr_pos;
            visited.insert(curr_pos);

            for direction in 0..8 {
                if direction & 1 != 0 {
                    continue;
                }

                let dest_result = self.test_step(curr_x, curr_y, curr_z, direction);
                if let Some(dest_z) = dest_result {
                    let (dest_x, dest_y) = Self::move_to(curr_x, curr_y, direction);
                    let dest_pos = (dest_x, dest_y, dest_z);

                    if (dest_x, dest_y, dest_z) == (dx, dy, dz) {
                        return Some(cur_gen);
                    }

                    if visited.contains(&dest_pos) {
                        continue;
                    }

                    if cur_gen + 1 < max_steps {
                        queue.push_front((dest_pos, cur_gen+1));
                    }
                } else {

                }

            }
        }

        None
    }


    pub fn trace_area(&self, x: isize, y: isize, z: i8, dx: isize, dy: isize, dz: i8, points: &mut Vec<Point>, options: &TraceOptions) {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut back_path = HashMap::new();
        let start_pos = Position(x, y, z);

        queue.push_front(start_pos);

        let start = Instant::now();

        let mut cnt = 0usize;

        let mut way_points = Vec::new();
        let mut way_points_index = QuadTree::new(0, 0, 8192, 8192);
        way_points.push(start_pos);
        way_points_index.insert(start_pos.0, start_pos.1);

        fn update_waypoints(wp: &mut QuadTree, pos: &Position) -> bool {
            let mut points = Vec::new();
            const D: isize = 512;
            wp.query_area(pos.0-D, pos.1-D, pos.0+D, pos.1+D, &mut points);
            points.len() == 0
        }

        while queue.len() > 0 {
            cnt += 1;
            let curr_pos = queue.pop_back().unwrap();
            // let curr_pos = queue.pop_front().unwrap();

            // if visited.contains(&curr_pos) {
            //     continue;
            // }
            let Position(curr_x, curr_y, curr_z) = curr_pos;

            if curr_x == dx && curr_y == dy {
                break;
            }
            // visited.insert(curr_pos);

            if update_waypoints(&mut way_points_index, &curr_pos) {
                way_points.push(curr_pos);
                way_points_index.insert(curr_pos.0, curr_pos.1);
            }

            if cnt % 100000 == 0 {
                println!("{cnt} {curr_pos:?}");

            }

            let dest_n = self.test_step_single(curr_x, curr_y, curr_z, 0);
            let dest_e = self.test_step_single(curr_x, curr_y, curr_z, 2);
            let dest_s = self.test_step_single(curr_x, curr_y, curr_z, 4);
            let dest_w = self.test_step_single(curr_x, curr_y, curr_z, 6);

            let steps = if false {
                [(0u8, dest_n),  (2, dest_e), (4, dest_s), (6, dest_w), (1, None), (3, None), (5, None), (7, None)]
            } else {
                let dest_ne = if dest_n.is_some() && dest_e.is_some() { self.test_step_single(curr_x, curr_y, curr_z, 1) } else { None };
                let dest_se = if dest_s.is_some() && dest_e.is_some() { self.test_step_single(curr_x, curr_y, curr_z, 3) } else { None };
                let dest_sw = if dest_s.is_some() && dest_w.is_some() { self.test_step_single(curr_x, curr_y, curr_z, 5) } else { None };
                let dest_nw = if dest_n.is_some() && dest_w.is_some() { self.test_step_single(curr_x, curr_y, curr_z, 7) } else { None };

                [(0, dest_n), (1, dest_ne), (2, dest_e), (3, dest_se), (4, dest_s), (5, dest_sw), (6, dest_w), (7, dest_nw)]
            };

            for (direction, dest_result) in steps {
                if let Some(dest_z) = dest_result {
                    let (dest_x, dest_y) = Self::move_to(curr_x, curr_y, direction);
                    let dest_pos = Position(dest_x, dest_y, dest_z);

                    if visited.contains(&dest_pos) {
                        continue;
                    }

                    visited.insert(dest_pos);
                    back_path.insert(dest_pos, curr_pos);
                    queue.push_front(dest_pos);
                }

            }
        }

        let duration = start.elapsed();
        println!("{cnt} {} {:?}", visited.len(), duration);

        for Position(x,y,z) in visited {
            points.push(Point {
                x,
                y,
                z,
                w: 0,
            })
        }


        // let mut curr_pos = Position(dx, dy, dz);
        // // let mut points = VecDeque::with_capacity(6144*2+4096*2);
        // cnt = 0;
        // println!("search path to back...");
        // loop {
        //     // println!("{cnt} {curr_pos:?}");
        //     let prev_pos = back_path[&curr_pos];
        //     points.push(Point{x: prev_pos.0, });
        //     cnt += 1;
        //     if prev_pos == start_pos {
        //         println!("found start!");
        //         break;
        //     }
        //     curr_pos = prev_pos;
        // }

        // let mut img = match image::open("map0.png") {
        //     Err(_) => ImageBuffer::new(6144, 4096),
        //     Ok(img) => img.to_rgb8(),
        // };
        // let mut img = ImageBuffer::new(6144, 4096);
        //
        // for Position(x, y, z) in visited.iter() {
        //     //img.put_pixel(*x as u32, *y as u32, Rgb([0, *z as u8 + 128, 0]));
        //     img.put_pixel(*x as u32, *y as u32, Rgb([0, (*z as i16).saturating_add(128) as u8, 0]));
        // }
        //
        // const R: u32 = 8;
        // cnt = 0;
        //
        // for Position(x, y, _z) in &way_points {
        //     let x = *x as u32;
        //     let y = *y as u32;
        //
        //
        //     for dx in -18..=18 {
        //         for dy in -18..=18 {
        //             let x= x as i32 + dx;
        //             let y = y as i32 + dy;
        //             if x >= 0 && x < 6144 && y >= 0 && y <= 4095 {
        //                 let (x, y) = (x as u32, y as u32);
        //
        //                 let &Rgb([r,g,b]) = img.get_pixel(x, y);
        //                 let r = if r == 0 { 64 } else { r.saturating_add(5) };
        //                 let g = if g == 0 { 0 } else { g.saturating_add(5) };
        //
        //                 img.put_pixel(x, y, Rgb([r,g,b]));
        //             }
        //         }
        //     }
        //     img.put_pixel(x, y, Rgb([255, 255, 255]));
        // }
        //
        // img.save("map0.png").unwrap();
        //
        // let Position(mut sx, mut sy, mut sz) = way_points.pop().unwrap();
        // let mut ordered_wp = Vec::new();
        // ordered_wp.push(Position(sx, sy, sz));
        //
        // if false {
        //     while way_points.len() > 0 {
        //         // println!("{} - {sx} {sy} {sz} => ...", way_points.len());
        //         let mut best_pos = Position(0,0,0);
        //         let mut best_len = 999999usize;
        //
        //         way_points.sort_by_key(|Position(x,y,_)| {
        //             (x-sx).abs().max((y-sy).abs())
        //         });
        //
        //         for Position(dx, dy, dz) in &way_points {
        //             let dd = (sx-dx).abs().max((sy-dy).abs()) as usize;
        //             // println!("{sx} {sy} {sz} -> {dx} {dy} {dz}... {dd}");
        //
        //             if dd < best_len {
        //                 if dd > 1000 {
        //                     best_len = dd;
        //                     best_pos = Position(*dx, *dy, *dz);
        //                 } else if let Some(len) = self.path_len(sx, sy, sz, *dx, *dy, *dz, best_len) {
        //                     if len < best_len {
        //                         // println!("{sx} {sy} {sz} -> {dx} {dy} {dz} = {len}");
        //                         best_len = len;
        //                         best_pos = Position(*dx, *dy, *dz);
        //                     }
        //                 }
        //             }
        //         }
        //
        //         way_points.retain(|x| *x != best_pos);
        //         ordered_wp.push(best_pos);
        //         println!("{}: {sx} {sy} {sz} => {best_pos:?} => {best_len}", way_points.len());
        //         let Position(dx, dy, _dz) = best_pos;
        //
        //         let (lx, ly) = ((dx-sx), (dy-sy));
        //         let max_l = lx.abs().max(ly.abs());
        //         let (step_x, step_y) = (lx as f32/ max_l as f32, ly as f32/ max_l as f32);
        //         let (mut fx, mut fy) = (sx as f32, sy as f32);
        //
        //         for _ in 0..max_l.abs() {
        //             let (x, y) = (fx as u32, fy as u32);
        //             if x < 6144 && y < 4096 {  // x and y are unsigned, so there is no need to check for numbers >= 0
        //                 img.put_pixel(x, y, Rgb([255, 0, 255]));
        //             }
        //             fx += step_x;
        //             fy += step_y;
        //         }
        //
        //         Position(sx, sy, sz) = best_pos;
        //     }
        //
        //     println!("{} waypoints", way_points.len());
        // }

        // use mul::server_items::SERVER_ITEMS;
        // for &(_serial, _graphic, _color, (x, y, z)) in SERVER_ITEMS {
        //     img.put_pixel(x as u32, y as u32, Rgb([255, 255, 255]));
        // }

        // for Position(x,y,z) in points {
        //     img.put_pixel(x as u32, y as u32, Rgb([255, (z as i16).saturating_add(128) as u8, 0]));
        // }
        //
        // img.save("map0.png").unwrap();
        //
        // let file = File::create("waypoints.txt").expect("Cannot create file waypoints.txt");
        // let mut writer = BufWriter::new(file);
        //
        // for Position(x, y, z) in ordered_wp.iter() {
        //     writeln!(writer, "{x} {y} {z}").expect("Cannot write to file waypoints.txt");
        // }

    }
}
