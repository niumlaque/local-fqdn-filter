#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum QueryType {
    UNKNOWN(u16),
    /// A host address
    A = 1,

    /// IP6 Address
    AAAA = 28,
}

impl From<QueryType> for u16 {
    fn from(value: QueryType) -> Self {
        use QueryType::*;
        match value {
            UNKNOWN(v) => v,
            A => 1,
            AAAA => 28,
        }
    }
}

impl From<u16> for QueryType {
    fn from(value: u16) -> Self {
        match value {
            1 => QueryType::A,
            28 => QueryType::AAAA,
            _ => QueryType::UNKNOWN(value),
        }
    }
}
