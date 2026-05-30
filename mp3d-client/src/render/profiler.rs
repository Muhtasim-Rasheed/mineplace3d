#[derive(Debug)]
pub struct Profiler {
    pub entries: Vec<ProfilerEntry>,
    pub smoothed_entries: Vec<ProfilerEntry>,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            smoothed_entries: Vec::new(),
        }
    }

    pub fn start_scope(&mut self, name: &'static str) -> ProfileScope<'_> {
        ProfileScope {
            profiler: self,
            name,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn begin_frame(&mut self) {
        self.entries.clear();
    }

    pub fn end_frame(&mut self) {
        for (i, entry) in self.entries.iter().enumerate() {
            if let Some(smoothed_entry) = self.smoothed_entries.get(i) {
                let duration = smoothed_entry.duration.mul_f32(0.9) + entry.duration.mul_f32(0.1);
                self.smoothed_entries[i] = ProfilerEntry {
                    name: entry.name,
                    duration,
                };
            } else {
                self.smoothed_entries.push(*entry);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfilerEntry {
    pub name: &'static str,
    pub duration: std::time::Duration,
}

impl PartialOrd for ProfilerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ProfilerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.duration.cmp(&other.duration)
    }
}

#[derive(Debug)]
pub struct ProfileScope<'a> {
    profiler: &'a mut Profiler,
    name: &'static str,
    start_time: std::time::Instant,
}

impl<'a> Drop for ProfileScope<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.profiler.entries.push(ProfilerEntry {
            name: self.name,
            duration,
        });
    }
}
