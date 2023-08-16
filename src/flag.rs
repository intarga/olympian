#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Flag {
    Pass,
    Fail,
    Warn,
    Inconclusive,
    Invalid,
    DataMissing,
    Isolated,
}
