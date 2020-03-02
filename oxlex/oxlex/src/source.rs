pub trait Source<'source>: Clone {
    fn len(&self) -> usize;
    fn get_at(&self, index: usize) -> &'source str;
    fn get_slice(&self, index: usize, until: usize) -> &'source str;
}

impl<'source> Source<'source> for &'source str {
    fn len(&self) -> usize {
        (*self).len()
    }

    fn get_at(&self, index: usize) -> &'source str {
        self.get(index..index+1).unwrap()
    }

    fn get_slice(&self, index: usize, until: usize) -> &'source str {
        self.get(index..until).unwrap()
    }
}
