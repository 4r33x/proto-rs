pub trait RepeatedCollection<T>: Sized {
    /// The iterator type returned by `.iter()` or `.into_iter()`.
    ///
    /// This associated type is parameterized by how `self` is accessed.
    /// - For `Self` (owned), the iterator yields `T`.
    /// - For `&Self`, it yields `&T`.
    /// - For `&mut Self`, it could yield `&mut T` if desired.
    type Iter<'a>: Iterator<Item = <Self::Item<'a> as std::ops::Deref>::Target>
    where
        Self: 'a,
        T: 'a;

    /// The item type produced by iteration, parameterized by `self`'s lifetime form.
    /// Typically:
    /// - For `Self`, this is `T`
    /// - For `&Self`, this is `&'a T`
    /// - For `&mut Self`, this is `&'a mut T`
    type Item<'a>: std::ops::Deref<Target = T> + 'a
    where
        Self: 'a,
        T: 'a;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Shared iteration (`&self`).
    fn iter(&self) -> Self::Iter<'_>;

    /// Consuming iteration (`self`).
    fn into_iter(self) -> Self::Iter<'static>;
}

pub trait RepeatedCollectionMut<T>: RepeatedCollection<T> + FromIterator<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().collect()
    }
    fn new_reserved(capacity: usize) -> Self;
    fn push(&mut self, value: T);
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for value in iter {
            self.push(value);
        }
    }
}
