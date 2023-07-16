/// Error performing non-blocking send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum SendErrorNoWait<T> {
    /// The channel is full.
    #[cfg_attr(feature = "thiserror", error("Full"))]
    Full(T),

    /// The channel is closed.
    #[cfg_attr(feature = "thiserror", error("Closed"))]
    Closed(T),
}

/// Error performing blocking send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum SendError<T> {
    /// The channel is closed
    #[cfg_attr(feature = "thiserror", error("Closed"))]
    Closed(T),
}

/// Error performing non-blocking recv.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum RecvErrorNoWait {
    /// The channel is empty.
    #[cfg_attr(feature = "thiserror", error("Empty"))]
    Empty,

    /// The channel is closed.
    #[cfg_attr(feature = "thiserror", error("Full"))]
    Closed,
}

/// Error performing blocking recv.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum RecvError {
    /// The channel is closed.
    #[cfg_attr(feature = "thiserror", error("Full"))]
    Closed,
}

impl<T> SendErrorNoWait<T> {
    /// Constructs [`SendErrorNoWait::Full`]
    pub fn full(value: T) -> Self {
        Self::Full(value)
    }

    /// Constructs [`SendErrorNoWait::Closed`]
    pub fn closed(value: T) -> Self {
        Self::Closed(value)
    }

    /// Check whether is [`SendErrorNoWait::Full`]
    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full { .. })
    }

    /// Check whether is [`SendErrorNoWait::Closed`]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl RecvErrorNoWait {
    /// Constructs [`RecvErrorNoWait::Empty`]
    pub fn empty() -> Self {
        Self::Empty
    }

    /// Constructs [`RecvErrorNoWait::Closed`]
    pub fn closed() -> Self {
        Self::Closed
    }

    /// Check whether is [`RecvErrorNoWait::Empty`]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty { .. })
    }

    /// Check whether is [`RecvErrorNoWait::Closed`]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl<T> SendError<T> {
    /// Constructs [`SendError::Closed`]
    pub fn closed(value: T) -> Self {
        Self::Closed(value)
    }

    /// Check whether is [`SendError::Closed`]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl RecvError {
    /// Constructs [`RecvError::Closed`]
    pub fn closed() -> Self {
        Self::Closed
    }

    /// Check whether is [`RecvError::Closed`]
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}
