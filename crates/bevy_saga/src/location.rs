/// Source location for an AST node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenLocation {
    start: usize,
    end: usize,
}

impl TokenLocation {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<(usize, usize)> for TokenLocation {
    fn from((start, end): (usize, usize)) -> Self {
        Self { start, end }
    }
}

/// Source location for an AST node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePos {
    pub line: usize,
    pub col: usize,
}

impl SourcePos {
    pub fn min(self, other: Self) -> Self {
        if self.line < other.line {
            self
        } else if self.line > other.line {
            other
        } else if self.col < other.col {
            self
        } else {
            other
        }
    }

    pub fn max(self, other: Self) -> Self {
        if self.line > other.line {
            self
        } else if self.line < other.line {
            other
        } else if self.col > other.col {
            self
        } else {
            other
        }
    }
}
