/// Status codes for use with Span.SetStatus. These correspond to the status
/// codes used by gRPC defined here: https://github.com/googleapis/googleapis/blob/master/google/rpc/code.proto
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum StatusCode {
    OK = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,
}

impl Default for StatusCode {
    fn default() -> StatusCode {
        StatusCode::OK
    }
}
