#[derive(Clone)]
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

    pub fn copy_from_slice(&mut self, data: &[T]) {
        assert!(data.len() == self.size * self.size);
        self.elts.copy_from_slice(data);
    }

    pub fn get(&self, x: usize, y: usize) -> T {
        assert!(x < self.size);
        assert!(y < self.size);
        self.elts[y * self.size + x]
    }

    pub fn get_mut_ref(&mut self, x: usize, y: usize) -> &mut T {
        assert!(x < self.size);
        assert!(y < self.size);
        &mut self.elts[y * self.size + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) {
        assert!(x < self.size);
        assert!(y < self.size);
        self.elts[y * self.size + x] = value;
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn elts(&self) -> &[T] {
        &self.elts
    }
}

impl PartialEq for SquareArray<i8> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.elts == other.elts
    }
}
