
#[derive(Debug)]
pub struct QuadTree {
    left: isize,    // left <= x
    top: isize,     // top <= y
    right: isize,   // right > x
    bottom: isize,  // bottom > y
    content: TreeData,   // points or quads
}

#[derive(Debug)]
enum TreeData {
    Points(Vec<(isize, isize)>),
    Quads([Box<QuadTree>; 4]),
}

/*
    quadrants[4]
 nw 0 | 1  ne
    -----
 sw 3 | 2  se
 */



impl QuadTree {
    pub fn new(left: isize, top: isize, right: isize, bottom: isize) -> Self {
        QuadTree {
            left,
            top,
            right,
            bottom,
            content: TreeData::Points(vec![]),
        }
    }

    fn subdivide(&mut self) {
        match &self.content {
            TreeData::Points(points ) => {
                let left = self.left;
                let top = self.top;
                let right = self.right;
                let bottom = self.bottom;

                let mid_x = (left + right) / 2;
                let mid_y = (top + bottom) / 2;

                // assert!(left != mid_x && mid_x != right);
                // assert!(top != mid_y && mid_y != bottom);
                //
                let mut quads = [
                    Box::new(QuadTree::new(left, top, mid_x, mid_y)),      // nw
                    Box::new(QuadTree::new(mid_x, top, right, mid_y)),      // ne
                    Box::new(QuadTree::new(mid_x, mid_y, right, bottom)),      // se
                    Box::new(QuadTree::new(left, mid_y, mid_x, bottom)),      // sw
                ];

                for (x, y)  in points {
                    let index = self.quadrant(*x, *y);
                    quads[index].insert(*x, *y)
                }
                self.content = TreeData::Quads(quads);

            }

            TreeData::Quads(_) => {

            }

        }
    }

    fn quadrant(&self, x: isize, y: isize) -> usize {
        let mid_x = (self.left + self.right) / 2;
        let mid_y = (self.top + self.bottom) / 2;
        if x < mid_x {
            if y < mid_y {
                0   // nw
            } else {
                3   // sw
            }
        } else if y < mid_y {
            1   // ne
        } else {
            2   // se
        }
    }

    fn childs(&self) -> usize {
        match &self.content {
            TreeData::Points(points) => points.len(),
            TreeData::Quads(quads) => {
                let mut childs = 0;
                for quad in quads {
                    childs += quad.childs();
                }

                childs
            }
        }
    }

    pub fn insert(&mut self, x: isize, y: isize) {
        if x < self.left || x >= self.right || y < self.top || y >= self.bottom {
            return
        }
        let quadrant  = self.quadrant(x, y);

        match &mut self.content {
            TreeData::Points(points) => {
                const CAPACITY: usize = 8;
                // if the capacity of the node is not reached, or if node is non-subdivisable,
                // just add an element to our points
                if points.len() < CAPACITY || (self.left+1 == self.right && self.top+1 == self.bottom) {
                    points.push((x, y));
                } else {
                    //  otherwise subdivide the current node to 4 new subtrees and call insert recursively again
                    self.subdivide();
                    self.insert(x, y);
                }
            },

            TreeData::Quads(quads) => {
                quads[quadrant].insert(x, y);
            }
        }
    }

    pub fn delete(&mut self, x: isize, y: isize) -> bool {
        let index = self.quadrant(x, y);

        match &mut self.content {
            TreeData::Points(points) => {
                //points.retain(|(px, py)| x != *px || y != *py);
                // ignore if no such element is found
                if let Some(pos) = points.iter().position(|(px, py)| x == *px && y == *py) {
                    points.remove(pos);
                }

            }

            TreeData::Quads(quads) => {
                quads[index].delete(x, y);
                if self.childs() == 0 {
                    self.content = TreeData::Points(vec![]);
                }
            }
        }

        false
    }

    pub fn query_point(&self, x: isize, y: isize, result: &mut Vec<(isize, isize)>) {
        if x >= self.right || y >= self.bottom || x <= self.left || y <= self.top {
            return
        }

        match &self.content {
            TreeData::Points(points ) => {
                for (px, py) in points {
                    if *px == x && *py == y  {
                        result.push((*px, *py));
                    }
                }
            }

            TreeData::Quads(quads) => {
                let index = self.quadrant(x, y);
                quads[index].query_point(x, y, result);
            }
        }
    }

    pub fn query_area(&self, left: isize, top: isize, right: isize, bottom: isize, result: &mut Vec<(isize, isize)>) {
        if left >= self.right || top >= self.bottom || right <= self.left || bottom <= self.top {
            return
        }

        match &self.content {
            TreeData::Points(points ) => {
                for (x, y) in points {
                    if *x >= left && *x < right && *y >= top && *y < bottom {
                        result.push((*x, *y));
                    }
                }
            }

            TreeData::Quads(quads) => {
                for node in quads {
                    node.query_area(left, top, right, bottom, result);
                }
            }
        }
    }
}
