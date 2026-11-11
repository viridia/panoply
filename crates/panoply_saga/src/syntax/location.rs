/// Source location for an AST node.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TokenLocation {
    start: usize,
    end: usize,
}

impl TokenLocation {
    /// Create a new location.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Union of two locations. Computes a span that covers both locations.
    pub fn union(&self, other: impl Into<Self>) -> Self {
        let other: TokenLocation = other.into();
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// The start offset of the location.
    pub fn start(&self) -> usize {
        self.start
    }

    /// The end offset of the location
    pub fn end(&self) -> usize {
        self.end
    }
}

impl From<(usize, usize)> for TokenLocation {
    fn from((start, end): (usize, usize)) -> Self {
        Self { start, end }
    }
}
