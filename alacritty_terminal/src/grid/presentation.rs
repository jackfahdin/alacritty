//! Optional, bounded observations for hosts maintaining logical row identities.

use crate::grid::{Dimensions, Grid, GridCell};
use crate::index::Line;
use crate::term::cell::Flags;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPresentationStats {
    pub scroll_epoch: u64,
    /// Epoch immediately after the latest screen reset, before subsequent input.
    pub generation_scroll_epoch: u64,
    /// Soft-wrap endings removed from the retained prefix in this generation.
    pub discarded_wraps: u64,
}

#[derive(Clone, Debug, Default)]
pub(super) struct PresentationTracker {
    generation_scroll_epoch: u64,
    discarded_wraps: u64,
}

impl<T> Grid<T> {
    pub fn set_presentation_tracking(&mut self, enabled: bool) {
        self.presentation = enabled.then(|| PresentationTracker {
            generation_scroll_epoch: self.scroll_epoch,
            ..PresentationTracker::default()
        });
    }

    pub fn presentation_stats(&self) -> Option<GridPresentationStats> {
        self.presentation.as_ref().map(|tracking| GridPresentationStats {
            scroll_epoch: self.scroll_epoch,
            generation_scroll_epoch: tracking.generation_scroll_epoch,
            discarded_wraps: tracking.discarded_wraps,
        })
    }

    pub(crate) fn reset_presentation_generation(&mut self) {
        if let Some(tracking) = self.presentation.as_mut() {
            *tracking = PresentationTracker {
                generation_scroll_epoch: self.scroll_epoch,
                ..PresentationTracker::default()
            };
        }
    }
}

impl<T: GridCell> Grid<T> {
    /// Read rows before rotation or truncation destroys their wrap endings.
    pub(super) fn record_discarded_rows(&mut self, count: usize) {
        if self.presentation.is_none() || count == 0 {
            return;
        }
        let first = self.topmost_line().0;
        let wraps = (0..count)
            .filter(|offset| {
                self.raw[Line(first + *offset as i32)]
                    .last()
                    .is_some_and(|cell| cell.flags().contains(Flags::WRAPLINE))
            })
            .count() as u64;
        let tracking = self.presentation.as_mut().unwrap();
        tracking.discarded_wraps = tracking.discarded_wraps.wrapping_add(wraps);
    }
}

#[cfg(test)]
mod tests {
    use crate::grid::{Dimensions, Grid};
    use crate::index::{Column, Line};
    use crate::term::cell::{Cell, Flags};

    #[test]
    fn eviction_counts_wraps_before_rows_are_reused() {
        for history in [0, 2, 24] {
            let mut grid = Grid::<Cell>::new(2, 1, history);
            grid.set_presentation_tracking(true);
            for _ in 0..100 {
                grid[Line(0)][Column(0)].flags.insert(Flags::WRAPLINE);
                grid.scroll_up(&(Line(0)..Line(2)), 1);
            }
            assert_eq!(grid.presentation_stats().unwrap().discarded_wraps, (100 - history) as u64);
            grid.update_history(0);
            assert_eq!(grid.presentation_stats().unwrap().discarded_wraps, 100);
            assert_eq!(grid.history_size(), 0);
        }
    }

    #[test]
    fn resets_start_a_new_observation_generation_without_resetting_scroll_epoch() {
        let mut grid = Grid::<Cell>::new(2, 1, 0);
        assert!(grid.presentation_stats().is_none());
        grid.set_presentation_tracking(true);
        grid[Line(0)][Column(0)].flags.insert(Flags::WRAPLINE);
        grid.scroll_up(&(Line(0)..Line(2)), 1);
        grid.reset_presentation_generation();
        let stats = grid.presentation_stats().unwrap();
        assert_eq!(stats.scroll_epoch, 1);
        assert_eq!(stats.generation_scroll_epoch, 1);
        assert_eq!(stats.discarded_wraps, 0);
    }
}
