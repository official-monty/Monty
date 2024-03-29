pub trait MoveType: Copy + Default {
    fn is_same_action(self, other: Self) -> bool;

    fn ptr(&self) -> i32;

    fn set_ptr(&mut self, val: i32);

    fn policy(&self) -> f32;

    fn set_policy(&mut self, val: f32);
}

#[derive(Clone, Default)]
pub struct MoveList<T> {
    list: Vec<T>,
}

impl<T> std::ops::Deref for MoveList<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<T> std::ops::DerefMut for MoveList<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

impl<T> std::ops::Index<usize> for MoveList<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index]
    }
}

impl<T> std::ops::IndexMut<usize> for MoveList<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.list[index]
    }
}

impl<T> MoveList<T> {
    #[inline]
    pub fn push(&mut self, mov: T) {
        self.list.push(mov);
    }

    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        self.list.swap(a, b);
    }
}
