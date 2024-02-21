/// Provides access to a square array such that the array elements are rotated in one of
/// four cardinal orientations: 0, 90, 180 or 240 degrees. The underlying array elements are
/// not modified; instead, the rotation is carried out by transforming the array coordinates
/// upon access.
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
    /// Construct a new `RotatingSquareArray` from an array of elements.
    ///
    /// # Arguments
    ///
    /// * `size` - the dimensions of the square along once side.
    /// * `rotation` - the rotation of the array in 90-degree increments, valid values
    ///   are 0..=3.
    /// * `elts` - the elements of the array, a borrowed reference. The length of the array
    ///   must be `size` * `size`.
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

    /// Return the array entry at the given coordinates.
    pub fn get(&self, x: i32, y: i32) -> T {
        assert!((x as usize) < self.size);
        assert!((y as usize) < self.size);
        self.elts[(self.base_index + x * self.dx + y * self.dy) as usize]
    }
}

impl<'a> RotatingSquareArray<'a, i8> {
    /// Return the array entry at the given coordinates. This version accepts floating-point
    /// coordinates, the result is a linearly-interpolated value.
    pub fn get_interpolated(&self, x: f32, y: f32) -> f32 {
        let xc = x.clamp(0., self.size as f32 - 0.000001);
        let yc = y.clamp(0., self.size as f32 - 0.000001);

        let x0 = xc.floor();
        let x1 = xc.ceil();
        let y0 = yc.floor();
        let y1 = yc.ceil();

        let h00 = self.get(x0 as i32, y0 as i32) as f32;
        let h01 = self.get(x0 as i32, y1 as i32) as f32;
        let h10 = self.get(x1 as i32, y0 as i32) as f32;
        let h11 = self.get(x1 as i32, y1 as i32) as f32;

        let fx = xc - x0;
        let fy = yc - y0;
        let h0 = h00 * (1. - fx) + h10 * fx;
        let h1 = h01 * (1. - fx) + h11 * fx;
        h0 * (1. - fy) + h1 * fy
    }
}
