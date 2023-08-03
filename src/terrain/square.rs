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
