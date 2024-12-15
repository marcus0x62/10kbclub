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

use actix_web::HttpRequest;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display, fmt::Formatter};
use tracing::error;

pub mod analyzer;
pub mod cloudflare;
pub mod config;
pub mod database;
pub mod error;
pub mod relatedlinks;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum SortOptions {
    New,
    Size,
    Votes,
}

impl Display for SortOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SortOptions::New => write!(f, "New"),
            SortOptions::Size => write!(f, "Size"),
            SortOptions::Votes => write!(f, "Votes"),
        }
    }
}

#[derive(Serialize)]
pub struct PageLink {
    index: usize,
    uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Site {
    offset: usize,
    id: u32,
    url: String,
    size: String,
    related: u32,
}

pub fn get_client_ip(req: &HttpRequest) -> Result<String, String> {
    match (req.headers().get("x-real-ip"), req.peer_addr()) {
        (Some(xri), _) => {
            let Ok(str) = xri.to_str() else {
                let msg = format!("cannot convert {xri:?} to string");
                error!("{msg}");
                return Err(msg);
            };
            Ok(String::from(str))
        }
        (None, Some(peer_ip)) => Ok(peer_ip.ip().to_string()),
        _ => Err("could not get IP address".into()),
    }
}

pub fn get_page_links(
    page: usize,
    count: f32,
    paginate: f32,
    sortby: SortOptions,
) -> (Vec<PageLink>, String, String) {
    if count > paginate {
        let mut page_links = vec![];
        let pages = (count / paginate).ceil() as usize;

        for i in 1..=pages {
            if i != page {
                page_links.push(PageLink {
                    index: i,
                    uri: format!("/?paginate={paginate}&sortby={sortby}&page={i}"),
                });
            } else {
                page_links.push(PageLink {
                    index: i,
                    uri: "".into(),
                });
            }
        }

        let prev_link = if page > 1 {
            format!("/?paginate={paginate}&sortby={sortby}&page={}", page - 1)
        } else {
            "".into()
        };

        let next_link = if page < pages {
            format!("/?paginate={paginate}&sortby={sortby}&page={}", page + 1)
        } else {
            "".into()
        };

        (page_links, prev_link, next_link)
    } else {
        (vec![], "".into(), "".into())
    }
}
