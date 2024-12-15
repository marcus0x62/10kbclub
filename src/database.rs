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
use actix_web::{web, Result};
use r2d2_sqlite::SqliteConnectionManager;
use regex::Regex;
use rusqlite::params;
use std::{error::Error, path::PathBuf};
use tracing::info;

use crate::error::TenKbError;
use crate::relatedlinks::RelatedLink;
use crate::{Site, SortOptions};

pub type Pool = r2d2::Pool<SqliteConnectionManager>;

pub fn init_db(path: &PathBuf) -> Pool {
    if !path.exists() {
        panic!("database file {path:?} does not exist");
    }

    let manager = SqliteConnectionManager::file(path);
    let pool = match Pool::new(manager) {
        Ok(pool) => pool,
        Err(e) => panic!("unable to get database pool: {e:?}"),
    };

    let Ok(conn) = pool.clone().get() else {
        panic!("Unable to get conn to set foreign keys");
    };

    let mut statement = conn.prepare("PRAGMA foreign_keys = ON;").unwrap();
    if let Err(e) = statement.execute([]) {
        panic!("Unable to enable foreign key enforcement: {e:?}");
    }

    pool
}

pub fn get_sites(
    pool: &Pool,
    sortby: SortOptions,
    skip: usize,
    paginate: usize,
) -> Result<Vec<Site>, TenKbError> {
    let pool = pool.clone();

    let db_query = match sortby {
        SortOptions::Votes => {
            r#"SELECT site_ids.id, site_ids.url, sites.size,
                      (SELECT COUNT(*) FROM related WHERE related.id = site_ids.id) AS related,
                      (SELECT COUNT(*) FROM votes WHERE votes.id = site_ids.id) AS upvotes
               FROM site_ids LEFT JOIN sites
               WHERE site_ids.id = sites.id AND valid = true
               ORDER BY upvotes DESC, size ASC LIMIT ?,?"#
        }
        SortOptions::Size => {
            r#"SELECT site_ids.id, site_ids.url, sites.size,
                      (SELECT COUNT(*) FROM related WHERE related.id = site_ids.id) AS related
               FROM site_ids LEFT JOIN sites WHERE site_ids.id = sites.id AND valid = true
               ORDER BY size LIMIT ?,?"#
        }
        SortOptions::New => {
            r#"SELECT site_ids.id, site_ids.url, sites.size,
                      (SELECT COUNT(*) FROM related WHERE related.id = site_ids.id) AS related
               FROM site_ids LEFT JOIN sites WHERE site_ids.id = sites.id AND valid = true
               ORDER BY date_added LIMIT ?,?"#
        }
    };

    let mut offset = skip;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(db_query)?;

    let rows = statement.query_map([&skip, &paginate], |row| {
        offset += 1;
        let size: f64 = row.get(2)?;
        Ok(Site {
            offset,
            id: row.get(0)?,
            url: row.get(1)?,
            size: format!("{:0.3}", size / 1024.0),
            related: row.get(3)?,
        })
    })?;

    Ok(rows.filter_map(Result::ok).collect::<Vec<Site>>())
}

pub fn get_site_count(pool: &Pool) -> Result<usize, TenKbError> {
    let db_query = r#"SELECT COUNT(id) FROM sites WHERE valid = true;"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(db_query)?;
    let res = statement.query_map([], |row| row.get(0))?;

    let res = res.into_iter().next();
    match res {
        Some(Ok(c)) => Ok(c),
        Some(Err(e)) => Err(e)?,
        None => Err(TenKbError::Msg("Query returned no rows".into())),
    }
}

pub fn get_site_url(pool: &Pool, id: u32) -> Result<String, TenKbError> {
    let db_query = r#"SELECT url FROM site_ids WHERE id = ?;"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(db_query)?;
    let res = statement.query_map([&id], |row| row.get(0))?;

    let res = res.into_iter().next();
    match res {
        Some(Ok(c)) => Ok(c),
        Some(Err(e)) => Err(e)?,
        None => Err(TenKbError::Msg("Query returned no rows".into())),
    }
}

pub fn submit_site(pool: web::Data<Pool>, site: String) -> Result<(), TenKbError> {
    if check_site_active(&pool, &site)? {
        info!("site '{site}' is already active");
        return Err(TenKbError::Msg(format!(
            "site '{site}' is already in the database"
        )));
    }

    if check_site_blocked(&pool, &site)? {
        info!("site '{site}' is blocked");
        return Err(TenKbError::Msg(format!(
            "sorry! site '{site}' is blocked from submission"
        )));
    }

    if check_site_queued(&pool, &site)? {
        info!("site '{site}' is already queued for validation");
        return Err(TenKbError::Msg(format!(
            "site '{site}' is already pending validation"
        )));
    }

    let conn = pool.clone().get()?;

    let query = r#"INSERT INTO site_ids (url) VALUES (?);"#;
    let mut statement = conn.prepare(query)?;
    statement.execute([&site])?;

    let query = r#"INSERT INTO validation_queue (id, date_added, scan)
        VALUES ((SELECT id FROM site_ids WHERE url = ?), DATETIME(), true);"#;

    let mut statement = conn.prepare(query)?;
    statement.execute([&site])?;

    Ok(())
}

pub fn check_site_active(pool: &web::Data<Pool>, site: &String) -> Result<bool, TenKbError> {
    let query = r#"SELECT site_ids.id FROM site_ids LEFT JOIN sites
                   WHERE site_ids.id = sites.id AND site_ids.url = ? AND sites.valid = true;"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(query)?;

    let rows = statement.query_map([&site], |row| row.get::<usize, u32>(0))?;

    Ok(!rows.filter_map(Result::ok).collect::<Vec<u32>>().is_empty())
}

pub fn check_site_blocked(pool: &web::Data<Pool>, site: &String) -> Result<bool, TenKbError> {
    let query = r#"SELECT pattern FROM blocked_site_patterns;"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(query)?;

    let rows = statement.query_map([], |row| row.get::<usize, String>(0))?;

    for pattern in rows.filter_map(Result::ok).collect::<Vec<String>>() {
        let Ok(re) = Regex::new(&pattern[..]) else {
            continue;
        };

        if re.is_match(&site[..]) {
            info!("site '{site}' matched block pattern '{pattern}'");
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn check_site_queued(pool: &web::Data<Pool>, site: &String) -> Result<bool, TenKbError> {
    let query = r#"SELECT site_ids.id FROM site_ids LEFT JOIN validation_queue
                   WHERE validation_queue.id = site_ids.id AND site_ids.url = ?"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(query)?;

    let rows = statement.query_map([&site], |row| row.get::<usize, u32>(0))?;

    Ok(!rows.filter_map(Result::ok).collect::<Vec<u32>>().is_empty())
}

pub fn generate_id(pool: web::Data<Pool>, id: String) -> Result<(), TenKbError> {
    let query = r#"INSERT INTO voter_ids (uuid) VALUES (?);"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(query)?;
    statement.execute([&id])?;

    Ok(())
}

pub fn cast_vote(
    pool: web::Data<Pool>,
    voter_id: String,
    site_id: u32,
    vote: isize,
) -> Result<(), TenKbError> {
    let upsert_query = r#"INSERT INTO votes
                          VALUES (?, (SELECT id FROM voter_ids WHERE uuid = ?))
                          ON CONFLICT(id, voter_id) DO NOTHING;"#;
    let unvote_query = r#"DELETE FROM votes
                          WHERE id = ? AND voter_id = (SELECT id FROM voter_ids WHERE uuid = ?);"#;

    let conn = pool.clone().get()?;

    let mut statement = conn.prepare(if vote == 0 {
        unvote_query
    } else {
        upsert_query
    })?;

    statement.execute(params![&site_id, &voter_id])?;
    Ok(())
}

pub fn get_votes(pool: web::Data<Pool>, voter_id: String) -> Result<Vec<u32>, TenKbError> {
    let query = r#"SELECT * FROM votes
                   WHERE voter_id = (SELECT id FROM voter_ids WHERE uuid = ?);"#;

    let conn = pool.clone().get()?;
    let mut statement = conn.prepare(query)?;

    let rows = statement.query_map([&voter_id], |row| row.get::<usize, u32>(0))?;
    Ok(rows.filter_map(Result::ok).collect::<Vec<u32>>())
}

pub fn get_validation_queue(pool: &Pool) -> Result<Vec<String>, Box<dyn Error>> {
    let conn = pool.clone().get()?;

    let db_query = r#"SELECT site_ids.url FROM site_ids LEFT JOIN validation_queue
                      WHERE site_ids.id = validation_queue.id AND validation_queue.scan = true"#;

    let mut statement = conn.prepare(db_query)?;
    let rows = statement.query_map([], |row| row.get::<usize, String>(0))?;
    Ok(rows.filter_map(Result::ok).collect::<Vec<String>>())
}

pub fn mark_bad(pool: &Pool, site: &str) -> Result<(), Box<dyn Error>> {
    let conn = pool.clone().get()?;
    conn.execute(
        r#"UPDATE validation_queue SET scan = false
           WHERE id = (SELECT id FROM site_ids WHERE url = ?)"#,
        params![site],
    )?;

    Ok(())
}

pub fn mark_bad_size(pool: &Pool, site: &str, size: f64) -> Result<(), Box<dyn Error>> {
    log_validation_failure(
        pool,
        site,
        format!("size validation failed: site is {size} bytes"),
    )?;

    let conn = pool.clone().get()?;
    conn.execute(
        r#"UPDATE validation_queue SET scan = false WHERE id = (SELECT id from site_ids WHERE url = ?)"#,
        params![site],
    )?;
    Ok(())
}

pub fn mark_good(pool: &Pool, site: &str, size: f64) -> Result<(), Box<dyn Error>> {
    let pool = pool.clone();
    let conn = pool.clone().get()?;
    conn.execute(
        r#"DELETE from validation_queue WHERE id = (SELECT id FROM site_ids WHERE url = ?)"#,
        params![site],
    )?;

    conn.execute(
        r#"INSERT INTO sites (id, date_added, size, valid)
          VALUES((SELECT id FROM site_ids WHERE url = ?), DATETIME(), ?, true);"#,
        params![site, size],
    )?;

    Ok(())
}

pub fn get_related(pool: &Pool, site: u32) -> Result<Vec<RelatedLink>, TenKbError> {
    let conn = pool.clone().get()?;

    let db_query =
        r#"SELECT url, discussion_url, date, title, score, comments FROM related WHERE ID = ?"#;

    let mut statement = conn.prepare(db_query)?;

    let rows = statement.query_map([&site], |row| {
        Ok(RelatedLink {
            url: row.get(0)?,
            discussion_url: row.get(1)?,
            date: row.get(2)?,
            description: row.get(3)?,
            upvotes: row.get(4)?,
            comments: row.get(5)?,
        })
    })?;

    Ok(rows.filter_map(Result::ok).collect::<Vec<RelatedLink>>())
}

pub fn update_related(
    pool: &Pool,
    site: &str,
    related: Vec<RelatedLink>,
) -> Result<(), Box<dyn Error>> {
    let pool = pool.clone();
    let conn = pool.clone().get()?;
    conn.execute(
        r#"DELETE from related WHERE id = (SELECT id from site_ids WHERE url = ?);"#,
        params![site],
    )?;

    for link in related {
        conn.execute(
            r#"INSERT INTO related
               VALUES ((SELECT id FROM site_ids WHERE url = ?), ?, ?, ?, ?, ?, ?);"#,
            params![
                site,
                link.url,
                link.discussion_url,
                link.date,
                link.description,
                link.upvotes,
                link.comments,
            ],
        )?;
    }

    Ok(())
}

pub fn log_validation_failure(pool: &Pool, site: &str, msg: String) -> Result<(), Box<dyn Error>> {
    let pool = pool.clone();
    let conn = pool.clone().get()?;
    conn.execute(
        r#"INSERT INTO validation_log
           VALUES ((SELECT id FROM site_ids WHERE url = ?), DATETIME(), ?)"#,
        params![site, msg],
    )?;

    Ok(())
}
