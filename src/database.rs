use rusqlite::{Connection, Result, NO_PARAMS, params};

use crate::PROCESSES;
use crate::net::Process;

pub fn create_db(filename: &String) -> Result<()> {
    let conn = Connection::open(filename)?;

    conn.execute(
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
    conn.execute(
        "CREATE TABLE protocols (
            prot_id     INTEGER PRIMARY KEY ASC,
            prot_name   TEXT NOT NULL DEFAULT ''
        );",
        NO_PARAMS,
    )?;
    conn.execute(
        "CREATE TABLE dates (
            date_id     INTEGER PRIMARY KEY ASC,
            date_str    TEXT NOT NULL
        );",
        NO_PARAMS,
    )?;
    conn.execute(
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

pub fn update_db(filename: &String, procs: Vec<Process>) -> Result<()> {
    let mut conn = Connection::open(filename)?;
    let transaction = conn.transaction().unwrap();
    let date = "07032021";

    transaction.execute(
        "INSERT INTO dates (date_str) VALUES (?1)",
        params![date],
    )?;

    for p in procs {
        let (pid, name, tlinks, ulinks, rx, tx) = p.get_all_info();

        transaction.execute(
            "INSERT INTO processes (p_pid, p_date_id, p_name, p_rx, p_tx)
             VALUES (?1, (SELECT date_id FROM dates WHERE date_str=?2), ?3, ?4, ?5)
             ON CONFLICT(p_pid, p_date_id) DO UPDATE SET p_rx = p_rx+?4, p_tx = p_tx+?5",
            params![pid, date, name, rx, tx]
        )?;

        //for t in p.get_tlinks() {
        //    let (saddr, daddr, sport, dport, rx, tx, prot, domain) = t.get_all_info();

        //    // TODO: finish and test request
        //    transaction.execute(
        //        "INSERT INTO links (l_p_pid, l_date_id,
        //            l_saddr, l_daddr, l_lport, l_dport, l_rx, l_tx, l_prot_id, l_domain)
        //         VALUES (?1, (SELECT date_id FROM dates WHERE date_str=?2),
        //            ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        //         ON CONFLICT(l_p_pid, l_date_id) DO UPDATE SET l_rx = l_rx+?7, l_tx = l_tx+?8",
        //        params![pid, date, saddr, daddr, lport, dport, rx, tx, domain]
        //    )?;
        //}
    }

    transaction.commit()
}

/*
 * TODO: TESTS
 */

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: init test: rm test.db

    #[test]
    fn test_create_db() -> Result<()> {
        let db_name = String::from("test.db");

        create_db(&db_name)
    }

    #[test]
    fn test_update_db() -> Result<()> {
        let db_name = String::from("test.db");

        let mut p0 = Process::new(1);
        p0.name(String::from("init"));
        p0.rx(10).tx(200);

        let mut p1 = Process::new(2);
        p1.name(String::from("systemd"));
        p1.rx(30).tx(400);

        let mut procs = vec![p0, p1];

        update_db(&db_name, procs)
    }
}
