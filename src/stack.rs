pub struct Stack {
    inner: Vec<Vec<u8>>,
    shrunk: usize,
    latest_minimum_instant: std::time::Instant,
    minimum_time_window: std::time::Duration,
    minimum_samples: Vec<usize>,
    minimum_samples_index: usize,
}

impl Stack {
    pub fn new(
        length: usize,
        minimum_time_window: std::time::Duration,
        minimum_samples: usize,
    ) -> Self {
        let mut inner = Vec::with_capacity(length);
        for _ in 0..length {
            inner.push(Vec::new());
        }
        Self {
            inner,
            shrunk: length,
            latest_minimum_instant: std::time::Instant::now(),
            minimum_time_window,
            minimum_samples: vec![length; minimum_samples],
            minimum_samples_index: 0,
        }
    }

    fn update_samples(&mut self) {
        while self.latest_minimum_instant.elapsed() >= self.minimum_time_window {
            self.minimum_samples_index =
                (self.minimum_samples_index + 1) % self.minimum_samples.len();
            self.minimum_samples[self.minimum_samples_index] = self.inner.len();
            self.latest_minimum_instant += self.minimum_time_window;
        }
    }

    pub fn push(&mut self, buffer: Vec<u8>) {
        self.update_samples();
        self.inner.push(buffer);
    }

    pub fn pop(&mut self) -> Option<Vec<u8>> {
        self.update_samples();
        let buffer = self.inner.pop();
        if buffer.is_some() {
            self.shrunk = self.shrunk.min(self.inner.len());
            self.minimum_samples[self.minimum_samples_index] =
                self.minimum_samples[self.minimum_samples_index].min(self.inner.len());
        }
        buffer
    }

    pub fn shrink_unused(&mut self) {
        self.update_samples();
        let running_minimum = *self
            .minimum_samples
            .iter()
            .min()
            .expect("minimum_samples is not empty");
        if running_minimum > self.shrunk {
            for buffer in self.inner.as_mut_slice()[self.shrunk..running_minimum].iter_mut() {
                buffer.clear();
                buffer.shrink_to_fit();
            }
            self.shrunk = running_minimum;
        }
    }
}
