/// Trait to convert an expression from dynamically typed to a known static type.
pub trait Coerce<T> {
    fn coerce(&self) -> Option<T>;
}
