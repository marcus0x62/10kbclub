// MIT License
//
// Copyright (c) 2024 Marcus Butler
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{
    convert::From,
    fmt::{Display, Formatter, Result},
};

use actix_web::{error::BlockingError, http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;

#[derive(Debug)]
pub enum TenKbError {
    Msg(String),
}

impl From<BlockingError> for TenKbError {
    fn from(err: BlockingError) -> Self {
        Self::Msg(err.to_string())
    }
}

impl From<r2d2::Error> for TenKbError {
    fn from(err: r2d2::Error) -> Self {
        Self::Msg(err.to_string())
    }
}

impl From<rusqlite::Error> for TenKbError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Msg(err.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct HtmlError {
    code: u16,
    status: String,
}

impl ResponseError for HtmlError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::from_u16(self.code).unwrap()).body(self.status.clone())
    }
}

impl Display for HtmlError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.status)
    }
}

impl From<TenKbError> for HtmlError {
    fn from(err: TenKbError) -> Self {
        match err {
            TenKbError::Msg(str) => HtmlError {
                code: 500,
                status: str.clone(),
            },
        }
    }
}

impl From<actix_web::Error> for HtmlError {
    fn from(err: actix_web::Error) -> Self {
        HtmlError {
            code: 500,
            status: err.to_string(),
        }
    }
}

impl From<BlockingError> for HtmlError {
    fn from(err: BlockingError) -> Self {
        Self {
            code: 500,
            status: err.to_string(),
        }
    }
}

impl From<String> for HtmlError {
    fn from(err: String) -> Self {
        HtmlError {
            code: 500,
            status: err,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JsonError {
    code: u16,
    status: String,
}

impl ResponseError for JsonError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::from_u16(self.code).unwrap()).json(self)
    }
}

impl Display for JsonError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let err_json = serde_json::to_string(self).unwrap();
        write!(f, "{}", err_json)
    }
}

impl From<&'static str> for JsonError {
    fn from(msg: &'static str) -> Self {
        JsonError {
            code: 500,
            status: msg.into(),
        }
    }
}

impl From<String> for JsonError {
    fn from(msg: String) -> Self {
        JsonError {
            code: 500,
            status: msg,
        }
    }
}

impl From<BlockingError> for JsonError {
    fn from(msg: BlockingError) -> Self {
        JsonError {
            code: msg.status_code().into(),
            status: msg.to_string(),
        }
    }
}

impl From<r2d2::Error> for JsonError {
    fn from(err: r2d2::Error) -> Self {
        JsonError {
            code: 500,
            status: err.to_string(),
        }
    }
}

impl From<rusqlite::Error> for JsonError {
    fn from(err: rusqlite::Error) -> Self {
        JsonError {
            code: 500,
            status: err.to_string(),
        }
    }
}

impl From<TenKbError> for JsonError {
    fn from(err: TenKbError) -> Self {
        match err {
            TenKbError::Msg(str) => Self {
                code: 500,
                status: str.clone(),
            },
        }
    }
}
