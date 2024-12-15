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
use std::error::Error;

use tenkbclub::{
    cloudflare::urlscan,
    config::Config,
    database::{get_validation_queue, init_db, mark_bad, mark_bad_size, mark_good},
};
use tokio::runtime::Handle;
use tracing::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load("/home/marcusb/code/10kbclub/config.json")?;

    let pool = init_db(&config.database_path);

    let mut first = true;

    loop {
        if !first {
            tokio::time::sleep(std::time::Duration::from_secs(1800)).await;
        }

        first = false;

        let sites = match get_validation_queue(&pool) {
            Ok(sites) => sites,
            Err(e) => {
                error!("unable to get site list: {e:?}");
                continue;
            }
        };

        for site in sites {
            match site_live(&site[..]).await {
                Ok(_) => {}
                Err(e) => {
                    error!("unable to retrieve {site}: {e:?}");
                    mark_bad(&pool, &site[..])?;
                    continue;
                }
            }

            match urlscan(&site[..], Handle::current(), &config).await {
                Ok(url) if url.acceptable => {
                    mark_good(&pool, &site[..], url.size)?;
                }
                Ok(url) => {
                    mark_bad_size(&pool, &site[..], url.size)?;
                }
                Err(e) => {
                    error!("unable to scan {site}: {e:?}");
                    mark_bad(&pool, &site[..])?;
                }
            }
        }
    }
}

async fn site_live(url: &str) -> Result<(), Box<dyn Error>> {
    let req = reqwest::get(url).await?;
    if req.status() != 200 {
        Err(format!("status code is {}", req.status()).into())
    } else {
        Ok(())
    }
}
