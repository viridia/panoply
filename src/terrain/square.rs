pub struct SquareArray<T> {
    size: usize,
    elts: Vec<T>,
}

impl<T: Clone> SquareArray<T>
where
    T: Copy,
{
    pub fn new(size: usize, fill: T) -> SquareArray<T> {
        Self {
            size,
            elts: vec![fill; size * size],
        }
    }

    pub fn from_slice(&mut self, data: &[T]) {
        assert!(data.len() == self.size * self.size);
        self.elts.copy_from_slice(data);
    }

    pub fn get(&self, x: i32, y: i32) -> T {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        self.elts[(y as usize * self.size) + x as usize]
    }

    pub fn get_mut_ref(&mut self, x: i32, y: i32) -> &mut T {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        &mut self.elts[(y as usize * self.size) + x as usize]
    }

    pub fn set(&mut self, x: i32, y: i32, value: T) {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        self.elts[(y as usize * self.size) + x as usize] = value;
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn elts(&self) -> &[T] {
        &self.elts
    }
}

pub struct RotatingSquareArray<'a, T> {
    size: usize,
    base_index: i32,
    dx: i32,
    dy: i32,
    elts: &'a [T],
}

impl<'a, T: Clone> RotatingSquareArray<'a, T>
where
    T: Copy,
{
    pub fn new(size: usize, rotation: i32, elts: &'a [T]) -> RotatingSquareArray<T> {
        let dx: i32;
        let dy: i32;
        let base_index: i32;
        let sz = size as i32;
        assert!(elts.len() == size * size);

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
            elts,
        }
    }

    pub fn get(&self, x: i32, y: i32) -> T {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        self.elts[(self.base_index + x * self.dx + y * self.dy) as usize]
    }
}
