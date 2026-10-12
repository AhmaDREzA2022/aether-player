use aether_common::Track;

/// An ordered list of tracks plus a "current" position.
///
/// Behavior at the ends (no wrap-around in V1):
/// - `next_track` on the last track returns `None` and stays put.
/// - `prev_track` on the first track returns `None` and stays put.
#[derive(Debug, Default)]
pub struct Queue {
    tracks: Vec<Track>,
    current: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn extend(&mut self, tracks: impl IntoIterator<Item = Track>) {
        self.tracks.extend(tracks);
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    pub fn current(&self) -> Option<&Track> {
        self.current.and_then(|i| self.tracks.get(i))
    }

    /// Jump to a position. An out-of-range index returns `None` and changes nothing.
    pub fn set_current(&mut self, index: usize) -> Option<&Track> {
        if index >= self.tracks.len() {
            return None;
        }
        self.current = Some(index);
        self.current()
    }

    /// Move to the next track. With nothing selected yet, starts at the first.
    pub fn next_track(&mut self) -> Option<&Track> {
        let next = match self.current {
            None => 0,
            Some(i) => i + 1,
        };
        if next >= self.tracks.len() {
            return None;
        }
        self.current = Some(next);
        self.current()
    }

    pub fn prev_track(&mut self) -> Option<&Track> {
        match self.current {
            Some(i) if i > 0 => {
                self.current = Some(i - 1);
                self.current()
            }
            _ => None,
        }
    }

    /// Remove a track. If the current track is removed, "current" points at
    /// the track that slid into its place (or the new last track).
    pub fn remove(&mut self, index: usize) -> Option<Track> {
        if index >= self.tracks.len() {
            return None;
        }
        let removed = self.tracks.remove(index);
        self.current = match self.current {
            _ if self.tracks.is_empty() => None,
            Some(c) if index < c => Some(c - 1),
            Some(c) if index == c => Some(c.min(self.tracks.len() - 1)),
            other => other,
        };
        Some(removed)
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::track;

    fn queue_of(n: u32) -> Queue {
        let mut q = Queue::new();
        q.extend((0..n).map(track));
        q
    }

    #[test]
    fn a_new_queue_has_no_current_track() {
        let q = queue_of(3);
        assert_eq!(q.len(), 3);
        assert!(q.current().is_none());
    }

    #[test]
    fn next_track_starts_at_the_first_and_stops_at_the_end() {
        let mut q = queue_of(2);
        assert_eq!(q.next_track().unwrap().title, "t0");
        assert_eq!(q.next_track().unwrap().title, "t1");
        assert!(q.next_track().is_none());
        assert_eq!(q.current_index(), Some(1)); // stayed on the last track
    }

    #[test]
    fn prev_track_goes_back_and_stops_at_the_start() {
        let mut q = queue_of(3);
        q.set_current(2);
        assert_eq!(q.prev_track().unwrap().title, "t1");
        assert_eq!(q.prev_track().unwrap().title, "t0");
        assert!(q.prev_track().is_none());
        assert_eq!(q.current_index(), Some(0));
    }

    #[test]
    fn set_current_ignores_a_bad_index() {
        let mut q = queue_of(2);
        q.set_current(1);
        assert!(q.set_current(9).is_none());
        assert_eq!(q.current_index(), Some(1));
    }

    #[test]
    fn removing_before_current_shifts_the_index() {
        let mut q = queue_of(3);
        q.set_current(2);
        q.remove(0);
        assert_eq!(q.current().unwrap().title, "t2");
        assert_eq!(q.current_index(), Some(1));
    }

    #[test]
    fn removing_the_current_track_slides_the_next_one_in() {
        let mut q = queue_of(3);
        q.set_current(1);
        let removed = q.remove(1).unwrap();
        assert_eq!(removed.title, "t1");
        assert_eq!(q.current().unwrap().title, "t2");
    }

    #[test]
    fn removing_the_current_last_track_clamps_to_the_new_last() {
        let mut q = queue_of(3);
        q.set_current(2);
        q.remove(2);
        assert_eq!(q.current().unwrap().title, "t1");
    }

    #[test]
    fn removing_the_only_track_empties_the_queue() {
        let mut q = queue_of(1);
        q.set_current(0);
        q.remove(0);
        assert!(q.current().is_none());
        assert!(q.is_empty());
    }

    #[test]
    fn remove_out_of_range_returns_none() {
        let mut q = queue_of(1);
        assert!(q.remove(5).is_none());
    }

    #[test]
    fn clear_empties_everything() {
        let mut q = queue_of(3);
        q.set_current(1);
        q.clear();
        assert!(q.is_empty());
        assert!(q.current().is_none());
    }
}