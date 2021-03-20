use rusqlite::{Connection, Result, NO_PARAMS, params, Transaction, MappedRows};
use rusqlite::types::{FromSql, FromSqlResult, FromSqlError, ValueRef};
use chrono::Utc;

use crate::net::{Process, Link, Prot};
use std::net::{IpAddr};

///
/// Newtype pattern. Wrapper around net::IpAddr to implement a the FromSql trait on a foreign type.
///
struct IpAddrWrapper(IpAddr);

///
/// Example of impl of FromSql for cutom types in rusqlite/src/types/url.rs.
///
impl FromSql for IpAddrWrapper {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(s) => {
                let ip_str = std::str::from_utf8(s).map_err(|e| FromSqlError::Other(Box::new(e)))?;

                // We can do this because the FromStr trait is implemented for IpAddr
                let ip = match ip_str.contains('.') {
                    true => IpAddr::V4(ip_str.parse().unwrap()),
                    false => IpAddr::V6(ip_str.parse().unwrap()),
                };

                Ok(IpAddrWrapper(ip))
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub fn create_db(db_name: &String) -> Result<Connection> {
    let db = Connection::open(db_name).unwrap();

    db.execute(
        "CREATE TABLE processes (
            p_pid       INTEGER NOT NULL,
            p_date_id   TEXT NOT NULL,
            p_name      TEXT NOT NULL DEFAULT '',
            p_rx        INTEGER,
            p_tx        INTEGER,
            CONSTRAINT processes_fk_0 FOREIGN KEY (p_date_id) REFERENCES dates(date_id),
            PRIMARY KEY (p_pid, p_date_id)
        );",
        NO_PARAMS,
    )?;
    db.execute(
        "CREATE TABLE protocols (
            prot_id     INTEGER PRIMARY KEY ASC,
            prot_name   TEXT NOT NULL DEFAULT ''
        );",
        NO_PARAMS,
    )?;
    db.execute(
        "CREATE TABLE dates (
            date_id     INTEGER PRIMARY KEY ASC,
            date_str    TEXT UNIQUE NOT NULL
        );",
        NO_PARAMS,
    )?;
    db.execute(
        "CREATE TABLE links (
            l_p_pid     INTEGER,
            l_date_id   TEXT NOT NULL,
            l_saddr     TEXT NULL DEFAULT '',
            l_daddr     TEXT NULL DEFAULT '',
            l_lport     INTEGER,
            l_dport     INTEGER,
            l_rx        INTEGER,
            l_tx        INTEGER,
            l_prot_id   INTEGER,
            l_domain    TEXT NOT NULL DEFAULT '',
            CONSTRAINT links_fk_0 FOREIGN KEY (l_p_pid) REFERENCES processes(p_id),
            CONSTRAINT links_fk_1 FOREIGN KEY (l_date_id) REFERENCES dates(date_id),
            CONSTRAINT links_fk_2 FOREIGN KEY (l_prot_id) REFERENCES protocols(prot_id),
            PRIMARY KEY (l_p_pid, l_date_id, l_saddr, l_daddr, l_lport, l_dport)
        );",
        NO_PARAMS,
    )?;

    Ok(db)
}

pub fn open_db(db_name: &String) -> Result<Connection> {
    let db = Connection::open(db_name).unwrap();

    Ok(db)
}

///
/// Returns the number of rows changed.
///
fn insert_proc(transaction: &Transaction, p: &Process, date: &str) -> Result<usize> {
    let (pid, name, _tlinks, _ulinks, rx, tx) = p.get_all_info();

    let ret = transaction.execute(
        "INSERT INTO processes (p_pid, p_date_id, p_name, p_rx, p_tx)
         VALUES (?1, (SELECT date_id FROM dates WHERE date_str=?2), ?3, ?4, ?5)
         ON CONFLICT(p_pid, p_date_id) DO UPDATE SET p_rx = p_rx+?4, p_tx = p_tx+?5",
        params![pid, date, name, rx, tx]
    )?;

    Ok(ret)
}

///
/// Returns the number of rows changed.
///
fn insert_link(transaction: &Transaction, pid: u32, l: &Link, date: &str) -> Result<usize> {
    let (saddr, daddr, lport, dport, rx, tx, prot, domain) = l.get_all_info();

    let ret = transaction.execute(
        "INSERT INTO links (l_p_pid, l_date_id,
            l_saddr, l_daddr, l_lport, l_dport, l_rx, l_tx, l_prot_id, l_domain)
         VALUES (?1, (SELECT date_id FROM dates WHERE date_str=?2),
            ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(l_p_pid, l_date_id, l_saddr, l_daddr, l_lport, l_dport)
         DO UPDATE SET l_rx = l_rx+?7, l_tx = l_tx+?8",
        params![pid, date, saddr, daddr, lport, dport, rx, tx, prot, domain]
    )?;

    Ok(ret)
}

///
/// Add processes and links contained in 'procs' to the database 'db'.
///
pub fn update_db(db: &mut Connection, procs: &Vec<Process>) -> Result<()> {
    let date = Utc::now().format("%m%d%Y").to_string();
    let transaction = db.transaction().unwrap();

    transaction.execute(
        "INSERT INTO dates (date_str) VALUES (?1)",
        params![date],
    )?;

    for p in procs {
        insert_proc(&transaction, &p, &date)?;

        let pid = p.get_pid();
        for tl in p.get_tlinks() {
            insert_link(&transaction, pid, &tl, &date)?;
        }
    }

    transaction.commit()
}

fn get_links(db: &Connection, p: &mut Process) {
    let mut stmt = db.prepare_cached(
        "SELECT l.l_saddr, l.l_daddr, l.l_lport,
            l.l_dport, l.l_rx, l.l_tx, l.l_prot_id, l.l_domain
         FROM links l, dates
         WHERE l.l_p_pid = :pid AND dates.date_str = :date_str;"
    ).unwrap();

    let links = stmt.query_map_named(
        &[(":pid", &p.pid), (":date_str", &p.date.as_str())], |row| {
        let saddr: IpAddrWrapper = row.get(0)?;
        let daddr: IpAddrWrapper = row.get(1)?;

        Ok(
            Link {
                saddr: saddr.0,
                daddr: daddr.0,
                lport: row.get(2)?,
                dport: row.get(3)?,
                rx: row.get(4)?,
                tx: row.get(5)?,
                prot: row.get(6)?,
                domain: row.get(7)?,
            }
        )
    }).unwrap();

    for item in links {
        let link = item.unwrap();

        //println!("    link: {} {} {} {}", link.saddr, link.prot, link.rx, link.tx);

        match link.prot {
            Prot::TCP => p.tlinks.push(link),
            Prot::UDP => p.ulinks.push(link),
            _ => (),
        };
    }
    //println!("tlinks: {} ulinks: {}", p.tlinks.len(), p.ulinks.len());
}

pub fn get_procs(db: &Connection) -> () {
    let mut stmt = db.prepare(
        "SELECT p.p_pid, p.p_name, p.p_rx, p.p_tx, dates.date_str
         FROM processes p
         LEFT JOIN dates ON dates.date_id = p.p_date_id;"
    ).unwrap();

    let procs = stmt.query_map(NO_PARAMS, |row| {
        Ok( Process {
            pid: row.get(0)?,
            name: row.get(1)?,
            tlinks: Vec::new(),
            ulinks: Vec::new(),
            rx: row.get(2)?,
            tx: row.get(3)?,
            date: row.get(4)?,
        })
    }).unwrap();

    for item in procs {
        let mut p = item.unwrap();
        //println!("procs: {} {}", p, p.date);

        get_links(db, &mut p);
    }
}

/*
 * TESTS
 */

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::net::Prot;
    use std::path::Path;
    use std::fs;

    #[test]
    fn test_create_db() -> Result<()> {
        let db_name = String::from("test.db");

        if Path::new(&db_name).exists() {
            fs::remove_file(&db_name);
        }

        create_db(&db_name);

        Ok(())
    }

    #[test]
    fn test_insert_proc_and_link() {
        let db_name = String::from("test.db");
        let mut db = Connection::open(db_name).unwrap();
        let tx = db.transaction().unwrap();

        let mut p0 = Process::new(1);
        p0.name(String::from("init"));
        p0.rx(10).tx(200);

        let mut p1 = Process::new(2);
        p1.name(String::from("systemd"));
        p1.rx(30).tx(400);

        let mut p2 = Process::new(1);
        p2.name(String::from("init"));
        p2.rx(40).tx(500);

        let mut l0 = Link::new(
            IpAddr::V4( Ipv4Addr::new(127, 0, 0, 1) ),
            IpAddr::V4( Ipv4Addr::new(217, 1, 1, 0) ),
            12345,
            443,
        );
        l0.prot(Prot::TCP);
        l0.rx(12000);
        l0.tx(3400);
        l0.domain(String::from("somewhere.inthe.cloud"));

        let mut l1 = Link::new(
            IpAddr::V4( Ipv4Addr::new(127, 0, 0, 1) ),
            IpAddr::V4( Ipv4Addr::new(217, 1, 1, 0) ),
            12345,
            443,
        );
        l1.prot(Prot::TCP);
        l1.rx(50000);
        l1.tx(70);
        l0.domain(String::from("somewhere.inthe.cloud"));

        let pid = 2;
        let date = "07032021";
        tx.execute(
            "INSERT INTO dates (date_str) VALUES (?1)",
            params![date],
        );

        // Should add new procs
        assert_eq!(insert_proc(&tx, &p0, &date), Ok(1));
        assert_eq!(insert_proc(&tx, &p1, &date), Ok(1));

        // Should update existing one
        assert_eq!(insert_proc(&tx, &p2, &date), Ok(1));

        // Should add a new entry
        assert_eq!(insert_link(&tx, pid, &l0, &date), Ok(1));

        // Should update an existing entry
        assert_eq!(insert_link(&tx, pid, &l1, &date), Ok(1));

        tx.commit();
    }
}
