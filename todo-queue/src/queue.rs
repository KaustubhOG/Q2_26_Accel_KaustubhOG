use std::collections::VecDeque;

pub struct Queue<T> {
    items: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self { items: VecDeque::new() }
    }

    pub fn enqueue(&mut self, item: T) {
        self.items.push_back(item);
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<Vec<T>> for Queue<T> {
    fn from(v: Vec<T>) -> Self {
        Self { items: VecDeque::from(v) }
    }
}

impl<T> From<Queue<T>> for Vec<T> {
    fn from(q: Queue<T>) -> Self {
        q.items.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_order() {
        let mut q: Queue<i32> = Queue::new();
        q.enqueue(1);
        q.enqueue(2);
        q.enqueue(3);
        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
        assert_eq!(q.dequeue(), Some(3));
        assert_eq!(q.dequeue(), None);
    }

    #[test]
    fn len_and_is_empty() {
        let mut q: Queue<&str> = Queue::new();
        assert!(q.is_empty());
        q.enqueue("a");
        assert_eq!(q.len(), 1);
        q.dequeue();
        assert!(q.is_empty());
    }

    #[test]
    fn peek_does_not_remove() {
        let mut q: Queue<u64> = Queue::new();
        q.enqueue(42);
        assert_eq!(q.peek(), Some(&42));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn from_vec_preserves_order() {
        let q = Queue::from(vec![10, 20, 30]);
        let collected: Vec<&i32> = q.iter().collect();
        assert_eq!(collected, vec![&10, &20, &30]);
    }
}
