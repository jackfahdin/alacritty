//! Optional, bounded observations for hosts maintaining logical row identities.

use std::ops::Range;

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

/// Bounded union of rows mutated since the last drain, in current grid coordinates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GridPresentationChanges {
    pub rows: Option<Range<Line>>,
    pub full_scan: bool,
}

#[derive(Clone, Debug)]
pub(super) struct PresentationTracker {
    generation_scroll_epoch: u64,
    discarded_wraps: u64,
    dirty: Option<Range<i64>>,
    full_scan: bool,
    last_row: i64,
}

impl Default for PresentationTracker {
    fn default() -> Self {
        Self {
            generation_scroll_epoch: 0,
            discarded_wraps: 0,
            dirty: None,
            full_scan: false,
            last_row: i64::MIN,
        }
    }
}

impl<T> Grid<T> {
    pub fn set_presentation_tracking(&mut self, enabled: bool) {
        self.presentation = enabled.then(|| PresentationTracker {
            generation_scroll_epoch: self.scroll_epoch,
            full_scan: true,
            ..PresentationTracker::default()
        });
    }

    #[inline]
    pub(super) fn mark_presentation_row(&mut self, row: Line) {
        let Some(tracking) = self.presentation.as_mut() else { return };
        let row = i64::from(row.0);
        // A printable character can acquire the same row several times. Only
        // the first access needs to union bounds; drains and structural edits
        // invalidate this memo before that coordinate can describe new content.
        if tracking.last_row == row {
            return;
        }
        tracking.last_row = row;
        self.mark_presentation_rows(row, row + 1);
    }

    #[inline]
    pub(super) fn mark_presentation_rows(&mut self, start: i64, end: i64) {
        let Some(tracking) = self.presentation.as_mut() else { return };
        if tracking.full_scan || start >= end {
            return;
        }
        match tracking.dirty.as_mut() {
            Some(range) => {
                // Most accesses write another cell on the same row. Avoid
                // touching tracker storage again until the range expands.
                if start < range.start {
                    range.start = start;
                }
                if end > range.end {
                    range.end = end;
                }
            },
            None => tracking.dirty = Some(start..end),
        }
    }

    pub(super) fn mark_presentation_full(&mut self) {
        if let Some(tracking) = self.presentation.as_mut() {
            tracking.full_scan = true;
            tracking.dirty = None;
            tracking.last_row = i64::MIN;
        }
    }

    pub(super) fn scroll_presentation_up(&mut self, positions: usize, bottom: i32) {
        if let Some(tracking) = self.presentation.as_mut() {
            tracking.last_row = i64::MIN;
            if let Some(range) = tracking.dirty.as_mut() {
                range.start = range.start.saturating_sub(positions as i64);
                range.end = range.end.saturating_sub(positions as i64);
            }
        }
        self.mark_presentation_rows(i64::from(bottom) - positions as i64, i64::from(bottom));
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
                full_scan: true,
                ..PresentationTracker::default()
            };
        }
    }
}

impl<T: GridCell> Grid<T> {
    /// Draining mutations does not reset cumulative eviction statistics.
    pub fn take_presentation_changes(&mut self) -> Option<GridPresentationChanges> {
        let first = i64::from(self.topmost_line().0);
        let end = self.screen_lines() as i64;
        let tracking = self.presentation.as_mut()?;
        tracking.last_row = i64::MIN;
        let full_scan = std::mem::take(&mut tracking.full_scan);
        let rows = tracking.dirty.take().and_then(|range| {
            let start = range.start.max(first);
            let end = range.end.min(end);
            (start < end).then_some(Line(start as i32)..Line(end as i32))
        });
        Some(GridPresentationChanges { rows, full_scan })
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
    fn same_row_writes_are_observed_again_after_draining_and_initializing() {
        let mut grid = Grid::<Cell>::new(2, 1, 2);
        grid.set_presentation_tracking(true);
        grid.take_presentation_changes();
        for _ in 0..3 {
            grid[Line(1)][Column(0)].flags.insert(Flags::WRAPLINE);
            grid[Line(1)][Column(0)].flags.remove(Flags::WRAPLINE);
            assert_eq!(grid.take_presentation_changes().unwrap().rows, Some(Line(1)..Line(2)));
        }
        grid.initialize_all();
        assert!(grid.take_presentation_changes().unwrap().full_scan);
    }

    #[test]
    fn changes_follow_scrolled_rows_and_drain_without_resetting_totals() {
        let mut grid = Grid::<Cell>::new(2, 1, 2);
        grid.set_presentation_tracking(true);
        assert!(grid.take_presentation_changes().unwrap().full_scan);
        grid[Line(0)][Column(0)].flags.insert(Flags::WRAPLINE);
        grid.scroll_up(&(Line(0)..Line(2)), 1);
        assert_eq!(grid.take_presentation_changes().unwrap().rows, Some(Line(-1)..Line(2)));
        assert_eq!(grid.take_presentation_changes().unwrap().rows, None);
        for _ in 0..10_000 {
            grid.scroll_up(&(Line(0)..Line(2)), 1);
        }
        let before = grid.presentation_stats();
        assert_eq!(grid.take_presentation_changes().unwrap().rows, Some(Line(-2)..Line(2)));
        assert_eq!(grid.presentation_stats(), before);
        grid.scroll_down(&(Line(0)..Line(2)), 1);
        assert!(grid.take_presentation_changes().unwrap().full_scan);
    }

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
