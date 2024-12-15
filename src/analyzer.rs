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

use crate::{
    cloudflare::urlscan,
    config::Config,
    database::{get_validation_queue, mark_bad, mark_bad_size, mark_good, update_related, Pool},
    relatedlinks::{hackernews, lobsters, RelatedLink},
};
use tokio::runtime::Handle;
use tracing::{debug, error, info};

pub async fn analyzer(pool: &Pool, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut first = true;

    loop {
        if !first {
            info!("sleeping");
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }

        first = false;

        let sites = match get_validation_queue(pool) {
            Ok(sites) => sites,
            Err(e) => {
                error!("unable to get site list: {e:?}");
                continue;
            }
        };

        info!("processing {} sites in the validation queue", sites.len());

        for site in sites {
            info!("processing {site}");
            match site_live(&site[..]).await {
                Ok(_) => info!("live check succeeded for {site}"),
                Err(e) => {
                    error!("site_live check: unable to retrieve {site}: {e:?}; marking bad");
                    mark_bad(pool, &site[..])?;
                    continue;
                }
            }

            match urlscan(&site[..], Handle::current(), config).await {
                Ok(url) if url.acceptable => {
                    info!("urlscan complete for '{site}'; marking good");
                    mark_good(pool, &site[..], url.size)?;
                }
                Ok(url) => {
                    error!(
                        "site '{site}' exceeds max size (is '{}' bytes); marking bad",
                        url.size
                    );
                    mark_bad_size(pool, &site[..], url.size)?;
                    continue;
                }
                Err(e) => {
                    error!("urlscan check: unable to scan {site}: {e:?}; marking bad");
                    mark_bad(pool, &site[..])?;
                    continue;
                }
            }

            info!("retrieving related links for hacker news");
            let mut links = hackernews(&site, Handle::current()).await?;
            debug!("hn links: {links:?}");

            if links.len() > 5 {
                debug!("more than 5 links returned, truncating");
                links = links.into_iter().take(5).collect::<Vec<RelatedLink>>();
            }

            info!("retrieving related links for lobsters");
            let mut lobsters_links = lobsters(&site, Handle::current()).await?;
            debug!("lobsters links: {lobsters_links:?}");

            if lobsters_links.len() > 5 {
                debug!("more than 5 links retruned, truncating");
                lobsters_links = lobsters_links
                    .into_iter()
                    .take(5)
                    .collect::<Vec<RelatedLink>>();
            }

            links.extend(lobsters_links);

            debug!("combined links: {links:?}");

            info!("updating related links in database");
            update_related(pool, &site[..], links)?;
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
