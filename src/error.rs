use thiserror::Error;

// ── YellowstoneError ──────────────────────────────────────────────────────
// Returned from client / builder operations.

#[derive(Debug, Error)]
pub enum YellowstoneError {
    #[error("connection failed: {0}")]
    Connection(#[source] tonic::transport::Error),

    #[error("subscribe rpc failed: {0}")]
    Subscribe(#[source] tonic::Status),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("tls error: {0}")]
    Tls(String),

    #[error("max reconnect attempts ({0}) exceeded")]
    MaxReconnectsExceeded(u32),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl YellowstoneError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
}

// ── StreamError ───────────────────────────────────────────────────────────
// Surfaced through the UpdateStream async stream.

#[derive(Debug, Error, Clone)]
pub enum StreamError {
    #[error("grpc stream error: code={code:?} message={message}")]
    Grpc {
        code:    tonic::Code,
        message: String,
    },

    #[error("decode error: {0}")]
    Decode(String),

    #[error("stream permanently closed after {attempts} reconnect attempts")]
    PermanentlyClosed { attempts: u32 },
}

impl StreamError {
    pub fn decode(msg: impl Into<String>) -> Self {
        Self::Decode(msg.into())
    }
}

impl From<tonic::Status> for StreamError {
    fn from(s: tonic::Status) -> Self {
        Self::Grpc {
            code:    s.code(),
            message: s.message().to_owned(),
        }
    }
}
