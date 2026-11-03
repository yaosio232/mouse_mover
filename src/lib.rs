use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

pub type WorkerCancel = Mutex<Option<Arc<AtomicBool>>>;

pub const fn countdown_seconds(hours: u32, minutes: u32, seconds: u32) -> u32 {
    hours * 3600 + minutes * 60 + seconds
}

pub fn replace_worker_cancel(slot: &WorkerCancel, next: Arc<AtomicBool>) {
    if let Some(previous) = slot.lock().unwrap().replace(next) {
        previous.store(true, Ordering::SeqCst);
    }
}

pub fn cancel_worker(slot: &WorkerCancel) {
    if let Some(cancel) = slot.lock().unwrap().take() {
        cancel.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Stopped,
    Monitoring,
    Moving,
}

#[derive(Debug)]
pub struct ActivityState {
    pub status: Status,
    pub last_input: Instant,
    pub countdown_deadline: Option<Instant>,
    pub countdown_done: bool,
    countdown_seconds: u32,
    countdown_enabled: bool,
    idle_seconds: u32,
}

impl ActivityState {
    pub fn new(now: Instant) -> Self {
        Self {
            status: Status::Stopped,
            last_input: now,
            countdown_deadline: None,
            countdown_done: false,
            countdown_seconds: 0,
            countdown_enabled: false,
            idle_seconds: 180,
        }
    }
    pub fn set_idle_seconds(&mut self, seconds: u32) {
        self.idle_seconds = seconds.max(1);
    }
    pub fn start(&mut self, last_input: Instant, now: Instant) {
        self.status = Status::Monitoring;
        self.last_input = last_input;
        self.reset_countdown(now);
    }
    pub fn stop(&mut self, done: bool) {
        self.status = Status::Stopped;
        self.countdown_deadline = None;
        self.countdown_done = done;
    }
    pub fn input(&mut self, now: Instant) {
        self.last_input = now;
        if self.status == Status::Moving {
            self.status = Status::Monitoring;
        }
    }
    pub fn configure_countdown(&mut self, enabled: bool, seconds: u32, now: Instant) {
        self.countdown_enabled = enabled;
        self.countdown_seconds = seconds;
        self.reset_countdown(now);
    }
    fn reset_countdown(&mut self, now: Instant) {
        self.countdown_done = false;
        self.countdown_deadline = (self.countdown_enabled && self.countdown_seconds > 0)
            .then(|| now + Duration::from_secs(u64::from(self.countdown_seconds)));
    }
    pub fn tick(&mut self, now: Instant) -> bool {
        if self
            .countdown_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.stop(true);
            return false;
        }
        if self.status == Status::Monitoring
            && now.duration_since(self.last_input)
                >= Duration::from_secs(u64::from(self.idle_seconds))
        {
            self.status = Status::Moving;
            return true;
        }
        false
    }
}

pub fn build_azki_path() -> Vec<Point> {
    const S: i32 = 3;
    let (top, mid, base) = (0, 195, 390);
    let mut x = 0;
    let mut points = vec![Point { x: 0, y: 360 }];
    fn bezier(points: &mut Vec<Point>, c1: Point, c2: Point, end: Point, n: i32) {
        let start = *points.last().unwrap();
        for i in 0..=n {
            let t = f64::from(i) / f64::from(n);
            let u = 1.0 - t;
            let coord = |a: i32, b: i32, c: i32, d: i32| {
                (u.powi(3) * f64::from(a)
                    + 3.0 * u * u * t * f64::from(b)
                    + 3.0 * u * t * t * f64::from(c)
                    + t.powi(3) * f64::from(d)) as i32
            };
            points.push(Point {
                x: coord(start.x, c1.x, c2.x, end.x),
                y: coord(start.y, c1.y, c2.y, end.y),
            });
        }
    }
    fn line(points: &mut Vec<Point>, end: Point, n: i32) {
        let start = *points.last().unwrap();
        for i in 0..=n {
            points.push(Point {
                x: start.x + (end.x - start.x) * i / n,
                y: start.y + (end.y - start.y) * i / n,
            });
        }
    }
    let p = |x, y| Point { x, y };
    bezier(
        &mut points,
        p(x + 10 * S, base - 40 * S),
        p(x + 18 * S, top + 10 * S),
        p(x + 26 * S, top + 5 * S),
        25,
    );
    bezier(
        &mut points,
        p(x + 34 * S, top + 8 * S),
        p(x + 42 * S, mid),
        p(x + 50 * S, base - 8 * S),
        25,
    );
    bezier(
        &mut points,
        p(x + 44 * S, mid + 5 * S),
        p(x + 28 * S, mid - 5 * S),
        p(x + 52 * S, mid),
        20,
    );
    x += 288;
    bezier(
        &mut points,
        p(x - 10 * S, mid - 10 * S),
        p(x + 5 * S, top + 5 * S),
        p(x + 35 * S, top + 8 * S),
        20,
    );
    bezier(
        &mut points,
        p(x + 50 * S, top + 5 * S),
        p(x + 40 * S, mid + 10 * S),
        p(x + 8 * S, base - 5 * S),
        25,
    );
    bezier(
        &mut points,
        p(x + 20 * S, base),
        p(x + 45 * S, base - 2 * S),
        p(x + 58 * S, base - 8 * S),
        20,
    );
    x += 288;
    bezier(
        &mut points,
        p(x - 5 * S, base - 20 * S),
        p(x + 2 * S, mid),
        p(x + 5 * S, top + 8 * S),
        20,
    );
    for (end, n) in [
        (p(x + 8 * S, top + 8 * S), 12),
        (p(x + 8 * S, base - 5 * S), 28),
        (p(x + 8 * S, mid + 5 * S), 14),
        (p(x + 48 * S, top + 15 * S), 20),
        (p(x + 8 * S, mid + 5 * S), 20),
        (p(x + 50 * S, base - 8 * S), 22),
    ] {
        line(&mut points, end, n);
    }
    x += 72 * S;
    bezier(
        &mut points,
        p(x - 12 * S, base - 8 * S),
        p(x + 2 * S, mid + 8 * S),
        p(x + 8 * S, mid - 6 * S),
        22,
    );
    bezier(
        &mut points,
        p(x + 10 * S, mid + 12 * S),
        p(x + 10 * S, base - 15 * S),
        p(x + 14 * S, base - 6 * S),
        22,
    );
    let size = 14 * S;
    let center = p(x + 14 * S, top + 22 * S);
    let bottom = p(center.x, center.y + size);
    bezier(
        &mut points,
        p(x + 20 * S, base - 32 * S),
        p(center.x + 6 * S, top + 42 * S),
        bottom,
        24,
    );
    for i in 0..=64 {
        let t = std::f64::consts::PI - 2.0 * std::f64::consts::PI * f64::from(i) / 64.0;
        let hx = 16.0 * t.sin().powi(3);
        let hy = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
        points.push(p(
            center.x + (hx * f64::from(size) / 18.0) as i32,
            center.y - (hy * f64::from(size) / 18.0) as i32,
        ));
    }
    bezier(
        &mut points,
        p(x + 26 * S, top + 48 * S),
        p(x + 36 * S, mid - 2 * S),
        p(x + 56 * S, mid - 6 * S),
        24,
    );
    points
}

pub fn center_path(path: &[Point], bounds: Bounds) -> Vec<Point> {
    let min_x = path.iter().map(|p| p.x).min().unwrap_or(0);
    let max_x = path.iter().map(|p| p.x).max().unwrap_or(0);
    let min_y = path.iter().map(|p| p.y).min().unwrap_or(0);
    let max_y = path.iter().map(|p| p.y).max().unwrap_or(0);
    let dx = (bounds.width - (max_x - min_x)) / 2 - min_x;
    let dy = (bounds.height - (max_y - min_y)) / 2 - min_y;
    path.iter()
        .map(|p| Point {
            x: p.x + dx,
            y: p.y + dy,
        })
        .collect()
}

pub fn round_step(value: f64, applied: i32) -> i32 {
    value.round_ties_even() as i32 - applied
}
pub fn clamp(p: Point, b: Bounds) -> Point {
    Point {
        x: p.x.clamp(b.left, b.left + b.width - 1),
        y: p.y.clamp(b.top, b.top + b.height - 1),
    }
}
pub fn is_physical_mouse(flags: u32) -> bool {
    flags & 1 == 0
}
pub fn is_physical_keyboard(flags: u32) -> bool {
    flags & 0x10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(seconds: u64) -> Instant {
        Instant::now() + Duration::from_secs(seconds)
    }
    #[test]
    fn countdown_duration_includes_hours_minutes_and_seconds() {
        assert_eq!(countdown_seconds(3, 0, 0), 10_800);
        assert_eq!(countdown_seconds(0, 59, 59), 3_599);
    }

    #[test]
    fn idle_threshold_and_physical_input_resume_monitoring() {
        let base = at(0);
        let mut s = ActivityState::new(base);
        s.start(base, base);
        assert!(!s.tick(base + Duration::from_secs(179)));
        assert!(s.tick(base + Duration::from_secs(180)));
        s.input(base + Duration::from_secs(181));
        assert_eq!(s.status, Status::Monitoring);
        assert!(!s.tick(base + Duration::from_secs(360)));
        assert!(s.tick(base + Duration::from_secs(361)));
    }
    #[test]
    fn configurable_idle_threshold_controls_movement() {
        let base = at(0);
        let mut s = ActivityState::new(base);
        s.set_idle_seconds(5);
        s.start(base, base);
        assert!(!s.tick(base + Duration::from_secs(4)));
        assert!(s.tick(base + Duration::from_secs(5)));
    }
    #[test]
    fn countdown_runs_before_start_and_expires_as_full_stop() {
        let now = at(0);
        let mut s = ActivityState::new(now);
        s.configure_countdown(true, 5, now);
        assert!(!s.tick(now + Duration::from_secs(5)));
        assert!(s.countdown_done);
        assert_eq!(s.status, Status::Stopped);
    }
    #[test]
    fn countdown_zero_disabled_and_input_preserve_deadline() {
        let now = at(0);
        let mut s = ActivityState::new(now);
        s.configure_countdown(true, 600, now);
        s.start(now, now);
        let deadline = s.countdown_deadline;
        s.input(now + Duration::from_secs(1));
        assert_eq!(s.countdown_deadline, deadline);
        s.stop(false);
        assert!(s.countdown_deadline.is_none());
        s.configure_countdown(true, 0, now);
        assert!(s.countdown_deadline.is_none());
        s.configure_countdown(false, 30, now);
        assert!(s.countdown_deadline.is_none());
    }
    #[test]
    fn azki_path_has_446_samples_and_centering_ignores_origin() {
        let path = build_azki_path();
        assert_eq!(path.len(), 446);
        assert_eq!(path[0], Point { x: 0, y: 360 });
        assert_eq!(path[26], Point { x: 78, y: 15 });
        assert_eq!(path[26], path[27]);
        assert_eq!(*path.last().unwrap(), Point { x: 960, y: 177 });
        let a = center_path(
            &path,
            Bounds {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
            },
        );
        let b = center_path(
            &path,
            Bounds {
                left: 100,
                top: -100,
                width: 1920,
                height: 1080,
            },
        );
        assert_eq!(a, b);
    }
    #[test]
    fn every_azki_sample_matches_the_original_path() {
        let expected: Vec<Point> = include_str!("../tests/fixtures/azki-path.tsv")
            .lines()
            .map(|line| {
                let mut xy = line.split('\t');
                Point {
                    x: xy.next().unwrap().parse().unwrap(),
                    y: xy.next().unwrap().parse().unwrap(),
                }
            })
            .collect();
        assert_eq!(build_azki_path(), expected);
    }
    #[test]
    fn input_flags_ignore_injected_mouse_and_only_injected_keyboard() {
        assert!(!is_physical_mouse(1));
        assert!(is_physical_mouse(0));
        assert!(!is_physical_keyboard(0x10));
        assert!(is_physical_keyboard(0));
        assert!(is_physical_keyboard(1));
    }
    #[test]
    fn replacing_worker_token_does_not_revive_the_previous_worker() {
        let slot = WorkerCancel::new(None);
        let previous = Arc::new(AtomicBool::new(false));
        replace_worker_cancel(&slot, previous.clone());
        let next = Arc::new(AtomicBool::new(false));
        replace_worker_cancel(&slot, next.clone());
        assert!(previous.load(Ordering::SeqCst));
        assert!(!next.load(Ordering::SeqCst));
    }
    #[test]
    fn physical_input_cancels_auto_move_and_keeps_monitoring() {
        let now = at(0);
        let mut s = ActivityState::new(now);
        s.start(now - Duration::from_secs(180), now);
        assert!(s.tick(now));
        let deadline = now + Duration::from_secs(600);
        s.configure_countdown(true, 600, now);
        assert_eq!(s.countdown_deadline, Some(deadline));
        s.input(now + Duration::from_secs(1));
        assert_eq!(s.status, Status::Monitoring);
        assert_eq!(s.countdown_deadline, Some(deadline));
        assert!(!s.tick(now + Duration::from_secs(180)));
    }
    #[test]
    fn movement_rounds_even_and_clamps() {
        assert_eq!(round_step(2.5, 0), 2);
        assert_eq!(round_step(-2.5, 0), -2);
        assert_eq!(
            clamp(
                Point { x: -10, y: 120 },
                Bounds {
                    left: 0,
                    top: 0,
                    width: 100,
                    height: 100
                }
            ),
            Point { x: 0, y: 99 }
        );
    }
}
