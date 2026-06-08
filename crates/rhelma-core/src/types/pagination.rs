use serde::{Deserialize, Serialize};

/// Simple offset/limit pagination request (Rhelma v5.1).
///
/// This is intentionally minimal and transport-agnostic.
/// Cursor-based pagination can be built on top of this for APIs that need it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageRequest {
    /// Zero-based offset into the result set.
    pub offset: u64,
    /// Maximum number of items to return (must be >= 1 in normal mode).
    pub limit: u64,
}

impl PageRequest {
    /// Create a new pagination request without any normalization.
    pub fn new(offset: u64, limit: u64) -> Self {
        Self { offset, limit }
    }

    /// Normalize pagination constraints per Rhelma v5.1 rules.
    ///
    /// - `limit == 0` → default to 20 (or any chosen platform default).
    /// - `offset` is left as-is.
    pub fn normalized(&self) -> Self {
        const DEFAULT_LIMIT: u64 = 20;

        let limit = if self.limit == 0 {
            DEFAULT_LIMIT
        } else {
            self.limit
        };
        Self {
            offset: self.offset,
            limit,
        }
    }

    /// Compute next offset safely (`offset + limit`), returning `None` on overflow.
    pub fn next_offset(&self) -> Option<u64> {
        self.offset.checked_add(self.limit)
    }
}

impl Default for PageRequest {
    /// Default page:
    /// - `offset = 0`
    /// - `limit = 20`
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 20,
        }
    }
}

/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    /// Field `items`.
    pub items: Vec<T>,
    /// Field `total`.
    pub total: u64,
    /// Field `offset`.
    pub offset: u64,
    /// Field `limit`.
    pub limit: u64,
}

impl<T> Paginated<T> {
    /// Construct a new paginated response.
    pub fn new(items: Vec<T>, total: u64, offset: u64, limit: u64) -> Self {
        Self {
            items,
            total,
            offset,
            limit,
        }
    }

    /// Compute total number of pages.
    pub fn total_pages(&self) -> u64 {
        if self.limit == 0 || self.total == 0 {
            return 0;
        }

        let full_pages = self.total / self.limit;
        let has_partial = !self.total.is_multiple_of(self.limit);
        full_pages + if has_partial { 1 } else { 0 }
    }

    /// Current page index (zero-based).
    pub fn current_page(&self) -> u64 {
        if self.limit == 0 {
            0
        } else {
            self.offset / self.limit
        }
    }

    /// Whether more pages exist.
    pub fn has_next(&self) -> bool {
        (self.offset + self.items.len() as u64) < self.total
    }

    /// Safe computation of next page offset.
    pub fn next_offset(&self) -> Option<u64> {
        if !self.has_next() {
            return None;
        }
        self.offset.checked_add(self.limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_uses_default_limit() {
        let pr = PageRequest::new(0, 0).normalized();
        assert_eq!(pr.limit, 20);
    }

    #[test]
    fn total_pages_computes_correctly() {
        let p = Paginated {
            items: vec![1, 2, 3],
            total: 25,
            offset: 0,
            limit: 10,
        };
        assert_eq!(p.total_pages(), 3);
    }

    #[test]
    fn has_next_and_next_offset_work() {
        let p = Paginated {
            items: vec![1, 2, 3],
            total: 10,
            offset: 0,
            limit: 3,
        };
        assert!(p.has_next());
        assert_eq!(p.next_offset(), Some(3));
    }
}
