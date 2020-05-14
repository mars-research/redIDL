use std::result;
use std::convert;
use std::io;
use std::error;
use std::fmt;

extern crate syn;
extern crate fs_extra;

#[derive(Clone)]
pub struct Error;

pub type Result<T> = result::Result<T, Error>;

impl convert::From<io::Error> for Error {
    fn from(_: io::Error) -> Error {
        Error
    }
}

impl convert::From<syn::Error> for Error {
    fn from(_: syn::Error) -> Error {
        Error
    }
}

impl convert::From<fs_extra::error::Error> for Error {
    fn from(_: fs_extra::error::Error) -> Error {
        Error
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compiler Error")
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Compiler Error")
    }
}

impl error::Error for Error {}

pub fn _err_helper<T, E>(result: result::Result<T, E>, msg: &str) -> Result<T>
where
    Error: convert::From<E>,
    E: error::Error
{
    match result {
        Err(e) => {
            println!("Error: {} (Cause: \"{}\")", msg, e);
            Err(Error::from(e))
        }
        Ok(r) => Ok(r)
    }
}

#[macro_export]
macro_rules! try_with_msg {
    ($e:expr, $($arg:expr),+) => {
        crate::error::_err_helper($e, &format!($($arg),+))
    };
}

#[macro_export]
macro_rules! fail_with_msg {
    ($($arg:expr),+) => {
        {
            println!("Error: {}", &format!($($arg),+));
            return Err(crate::error::Error)
        }
    };
}
