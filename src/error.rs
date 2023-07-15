#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendErrorNoWait<T> {
    Full(T),
    Closed(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError<T> {
    Closed(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvErrorNoWait {
    Empty,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvError {
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
