use crate::errors::{Error, Result};
use enum_map::Enum;
use std::fmt::{Debug, Display, Formatter};
use std::num::NonZeroU16;
use std::rc::Rc;

pub type Text = Rc<str>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ident(Rc<str>);

impl Ident {
    pub fn is_reference(&self) -> bool {
        self.0.as_ref().starts_with('@')
    }
    pub fn is_value(&self) -> bool {
        !self.is_reference()
    }
    pub fn untagged(&self) -> String {
        if self.is_reference() {
            self.0.strip_prefix('@').unwrap().to_owned()
        } else {
            self.0.as_ref().to_owned()
        }
    }
}

impl From<&str> for Ident {
    fn from(s: &str) -> Self {
        Ident(s.into())
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.0.as_ref(), f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HttpStatusRange {
    Info,
    Success,
    Redirect,
    ClientError,
    ServerError,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HttpStatus {
    Code(NonZeroU16),
    Range(HttpStatusRange),
}

impl TryFrom<u64> for HttpStatus {
    type Error = Error;

    fn try_from(v: u64) -> Result<Self> {
        if (100..=599).contains(&v) {
            Ok(HttpStatus::Code(unsafe {
                NonZeroU16::new_unchecked(v.try_into().unwrap())
            }))
        } else {
            Err(Error::new("status not in range"))
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Primitive {
    Number,
    String,
    Boolean,
    Integer,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Enum)]
pub enum Method {
    Get,
    Put,
    Post,
    Patch,
    Delete,
    Options,
    Head,
}
