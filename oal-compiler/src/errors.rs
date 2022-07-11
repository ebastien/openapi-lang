use oal_syntax::span::Span;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Kind {
    Unknown,
    NotInScope,
    NotAFunction,
    InvalidTypes,
    Conflict,
    UnexpectedExpression,
    InvalidYAML,
    CycleDetected,
    IO,
    InvalidURL,
    InvalidHttpStatus,
    InvalidSyntax,
}

impl Default for Kind {
    fn default() -> Self {
        Kind::Unknown
    }
}

#[derive(Debug, Clone, Default)]
pub struct Error {
    pub kind: Kind,
    msg: String,
    details: Vec<String>,
    span: Option<Span>,
}

impl Error {
    pub fn new<S: Into<String>>(kind: Kind, msg: S) -> Error {
        Error {
            kind,
            msg: msg.into(),
            details: Vec::new(),
            span: None,
        }
    }

    pub fn with<T: Debug>(mut self, e: &T) -> Self {
        self.details.push(format!("{:?}", e));
        self
    }

    pub fn at(mut self, span: Option<Span>) -> Self {
        self.span = span;
        self
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{:?}: {}", self.kind, self.msg)?;
        if let Some(span) = &self.span {
            writeln!(f, "Location: {}", span)?;
        }
        if !self.details.is_empty() {
            writeln!(f, "Details:")?;
            self.details
                .iter()
                .try_for_each(|d| writeln!(f, " {}", d))?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::new(Kind::InvalidYAML, e.to_string())
    }
}

impl From<()> for Error {
    fn from(_: ()) -> Self {
        Error::new(Kind::Unknown, "")
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(Kind::IO, e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::new(Kind::InvalidURL, e.to_string())
    }
}

impl From<oal_syntax::errors::Error> for Error {
    fn from(e: oal_syntax::errors::Error) -> Self {
        Error::new(Kind::InvalidSyntax, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
