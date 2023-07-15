#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum SendErrorNoWait<T> {
    #[cfg_attr(feature = "thiserror", error("Full"))]
    Full(T),
    #[cfg_attr(feature = "thiserror", error("Closed"))]
    Closed(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum SendError<T> {
    #[cfg_attr(feature = "thiserror", error("Closed"))]
    Closed(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum RecvErrorNoWait {
    #[cfg_attr(feature = "thiserror", error("Empty"))]
    Empty,
    #[cfg_attr(feature = "thiserror", error("Full"))]
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum RecvError {
    #[cfg_attr(feature = "thiserror", error("Full"))]
    Closed,
}

impl<T> SendErrorNoWait<T> {
    pub fn full(value: T) -> Self {
        Self::Full(value)
    }
    pub fn closed(value: T) -> Self {
        Self::Closed(value)
    }

    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full { .. })
    }
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl RecvErrorNoWait {
    pub fn empty() -> Self {
        Self::Empty
    }
    pub fn closed() -> Self {
        Self::Closed
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty { .. })
    }
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl<T> SendError<T> {
    pub fn closed(value: T) -> Self {
        Self::Closed(value)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}

impl RecvError {
    pub fn closed() -> Self {
        Self::Closed
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed { .. })
    }
}
