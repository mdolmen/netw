use rusqlite::{Connection, Result, NO_PARAMS, params};
use rusqlite::{Transaction};
use std::path::Path;
use std::fs;

use crate::PROCESSES;
use crate::net::{Process, Link};

pub fn create_db(db_name: &String) -> Result<()> {
    let db = Connection::open(db_name)?;

    db.execute(
        "CREATE TABLE processes (
            p_pid       INTEGER NOT NULL,
            p_date_id   TEXT NOT NULL,
            p_name      TEXT NOT NULL DEFAULT '',
            p_rx        REAL,
            p_tx        REAL,
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
            date_str    TEXT NOT NULL
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
            l_rx        REAL,
            l_tx        REAL,
            l_prot_id   INTEGER,
            l_domain    TEXT NOT NULL DEFAULT '',
            CONSTRAINT links_fk_0 FOREIGN KEY (l_p_pid) REFERENCES processes(p_id),
            CONSTRAINT links_fk_1 FOREIGN KEY (l_date_id) REFERENCES dates(date_id),
            CONSTRAINT links_fk_2 FOREIGN KEY (l_prot_id) REFERENCES protocols(prot_id),
            PRIMARY KEY (l_date_id, l_saddr, l_daddr, l_lport, l_dport)
        );",
        NO_PARAMS,
    )?;

    Ok(())
}

///
/// Returns the number of rows changed.
///
fn insert_proc(transaction: &Transaction, p: &Process, date: &str) -> Result<usize> {
    let (pid, name, tlinks, ulinks, rx, tx) = p.get_all_info();

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
fn insert_link(transaction: &Transaction, p: &Link, date: &str) -> Result<()> {
    //let (saddr, daddr, sport, dport, rx, tx, prot, domain) = t.get_all_info();

    //// TODO: finish and test request
    //transaction.execute(
    //    "INSERT INTO links (l_p_pid, l_date_id,
    //        l_saddr, l_daddr, l_lport, l_dport, l_rx, l_tx, l_prot_id, l_domain)
    //     VALUES (?1, (SELECT date_id FROM dates WHERE date_str=?2),
    //        ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
    //     ON CONFLICT(l_p_pid, l_date_id) DO UPDATE SET l_rx = l_rx+?7, l_tx = l_tx+?8",
    //    params![pid, date, saddr, daddr, lport, dport, rx, tx, domain]
    //)?;
    Ok(())
}

pub fn update_db(db_name: &String, procs: &Vec<Process>) -> Result<()> {
    let mut db = Connection::open(db_name)?;
    let transaction = db.transaction().unwrap();

    // TODO: get today's date
    let date = "07032021";

    transaction.execute(
        "INSERT INTO dates (date_str) VALUES (?1)",
        params![date],
    )?;

    for p in procs {
        insert_proc(&transaction, &p, &date)?;

        for tl in p.get_tlinks() {
            insert_link(&transaction, tl, &date)?;
        }
    }

    transaction.commit()
}

/*
 * TESTS
 */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_db() -> Result<()> {
        let db_name = String::from("test.db");

        if Path::new(&db_name).exists() {
            fs::remove_file(&db_name);
        }

        create_db(&db_name)
    }

    #[test]
    fn test_insert_proc() {
        let db_name = String::from("test.db");
        let mut db = Connection::open(db_name).unwrap();
        let tx = db.transaction().unwrap();
        let date = "07032021";

        let mut p3 = Process::new(3);
        p3.name(String::from("nvim"));
        p3.rx(40).tx(500);

        tx.execute(
            "INSERT INTO dates (date_str) VALUES (?1)",
            params![date],
        );

        assert_eq!(insert_proc(&tx, &p3, &date), Ok(1));
    }

    #[test]
    fn test_update_db() {
        let db_name = String::from("test.db");

        let mut p0 = Process::new(1);
        p0.name(String::from("init"));
        p0.rx(10).tx(200);

        let mut p1 = Process::new(2);
        p1.name(String::from("systemd"));
        p1.rx(30).tx(400);

        let mut procs = vec![p0, p1];

        assert_eq!(update_db(&db_name, &procs), Ok(()))
    }
}
