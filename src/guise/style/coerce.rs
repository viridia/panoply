pub trait Coerce<T> {
    fn coerce(&self) -> Option<T>;
}
