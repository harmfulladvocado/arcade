use std::time::Duration;

const MELODY: &[(f32, u64)] = &[
    (392.0, 150), (440.0, 150), (494.0, 150), (523.0, 300),
    (0.0,   100), (494.0, 150), (440.0, 150), (392.0, 300),
    (0.0,   200), (349.0, 150), (392.0, 150), (440.0, 150),
    (392.0, 300), (0.0,   100), (330.0, 150), (392.0, 600),
    (0.0,   300),
];

struct MelodySource {
    note_idx: usize,
    sample_rate: u32,
    samples_done: u64,
    samples_total: u64,
}

impl MelodySource {
    fn new() -> Self {
        let sr = 44_100u32;
        Self {
            note_idx: 0,
            sample_rate: sr,
            samples_done: 0,
            samples_total: MELODY[0].1 * sr as u64 / 1000,
        }
    }
}

impl Iterator for MelodySource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.samples_done >= self.samples_total {
            self.note_idx = (self.note_idx + 1) % MELODY.len();
            self.samples_total = MELODY[self.note_idx].1 * self.sample_rate as u64 / 1000;
            self.samples_done = 0;
        }
        let (freq, _) = MELODY[self.note_idx];
        let t = self.samples_done as f32 / self.sample_rate as f32;
        self.samples_done += 1;
        Some(if freq > 0.0 {
            (2.0 * std::f32::consts::PI * freq * t).sin() * 0.15
        } else {
            0.0
        })
    }
}

impl rodio::Source for MelodySource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { None }
}

pub fn start() -> Option<(rodio::OutputStream, rodio::Sink)> {
    let (stream, handle) = rodio::OutputStream::try_default().ok()?;
    let sink = rodio::Sink::try_new(&handle).ok()?;
    sink.append(MelodySource::new());
    Some((stream, sink))
}