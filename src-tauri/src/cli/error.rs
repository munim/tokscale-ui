use thiserror::Error;

/// Errors returned by serialized `tokscale` CLI invocations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CliError {
    #[error("tokscale CLI binary not found")]
    NotFound,
    #[error("tokscale CLI command timed out")]
    Timeout,
    #[error("tokscale CLI exited with status {code:?}: {stderr}")]
    NonZeroExit { code: Option<i32>, stderr: String },
    #[error("tokscale CLI returned invalid JSON: {0}")]
    InvalidJson(String),
    #[error("tokscale CLI stdout exceeded 64 MiB limit")]
    OutputTooLarge,
}
