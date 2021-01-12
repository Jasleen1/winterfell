#[derive(Debug)]
pub enum PlasmaError {
    ConnectError(String),
    AlreadyExists,
    AlreadySealed,
    NotMutable,
    UnknownError(String),
}
