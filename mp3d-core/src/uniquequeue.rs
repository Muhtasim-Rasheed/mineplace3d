#[derive(Debug, Default)]
pub struct UniqueQueue<T> {
    queue: std::collections::VecDeque<T>,
    set: std::collections::HashSet<T>,
}

impl<T: std::hash::Hash + Eq + Copy> UniqueQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: std::collections::VecDeque::new(),
            set: std::collections::HashSet::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        if self.set.insert(item) {
            self.queue.push_back(item);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if let Some(item) = self.queue.pop_front() {
            self.set.remove(&item);
            Some(item)
        } else {
            None
        }
    }

    pub fn remove(&mut self, item: &T) {
        if self.set.remove(item)
            && let Some(pos) = self.queue.iter().position(|x| x == item)
        {
            self.queue.remove(pos);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn drain(&mut self, max: usize) -> Vec<T> {
        let mut items = Vec::new();
        for _ in 0..max {
            if let Some(item) = self.pop() {
                items.push(item);
            } else {
                break;
            }
        }
        items
    }
}
