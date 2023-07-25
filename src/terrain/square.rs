pub struct SquareArray<T> {
    size: usize,
    base_index: i32,
    dx: i32,
    dy: i32,
    elts: Vec<T>,
}

impl<T: Clone> SquareArray<T>
where
    T: Copy,
{
    pub fn new(size: usize, rotation: u8, fill: T) -> SquareArray<T> {
        let dx: i32;
        let dy: i32;
        let base_index: i32;
        let sz = size as i32;

        match rotation {
            0 => {
                dx = 1;
                dy = sz;
                base_index = 0;
            }

            1 => {
                dx = -sz;
                dy = 1;
                base_index = sz * (sz - 1);
            }

            2 => {
                dx = -1;
                dy = -(size as i32);
                base_index = sz * sz - 1;
            }

            3 => {
                dx = size as i32;
                dy = -1;
                base_index = sz - 1;
            }

            _ => {
                panic!("Invalid rotation!")
            }
        }

        Self {
            size,
            base_index,
            dx,
            dy,
            elts: vec![fill; size * size],
        }
    }

    pub fn get(&self, x: i32, y: i32) -> T {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        self.elts[(self.base_index + x * self.dx + y * self.dy) as usize]
    }

    pub fn set(&mut self, x: i32, y: i32, value: T) {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        self.elts[(self.base_index + x * self.dx + y * self.dy) as usize] = value;
    }

    // pub fn size(&self) -> usize {
    //     self.size
    // }
}
