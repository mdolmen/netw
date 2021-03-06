use rusqlite::{Connection, Result};
use rusqlite::NO_PARAMS;

use crate::net::Process;

pub fn create_db(filename: String) -> Result<()> {
    let conn = Connection::open(filename)?;

    conn.execute(
        "CREATE TABLE processes (
            p_id 		INTEGER PRIMARY KEY ASC,
            p_date      TEXT,
            p_pid       INTEGER NOT NULL,
            p_name 		TEXT NOT NULL DEFAULT '',
            p_rx        REAL,
            p_tx        REAL
        );",
        NO_PARAMS,
    )?;
    conn.execute(
        "CREATE TABLE protocols (
            prot_id 	INTEGER PRIMARY KEY ASC,
            prot_name 	TEXT NOT NULL DEFAULT ''
        );",
        NO_PARAMS,
    )?;
    conn.execute(
        "CREATE TABLE links (
            l_id		INTEGER PRIMARY KEY ASC,
            l_p_id		INTEGER,
            l_saddr		TEXT NULL DEFAULT '',
            l_daddr		TEXT NULL DEFAULT '',
            l_lport     INTEGER,
            l_dport     INTEGER,
            l_rx        REAL,
            l_tx        REAL,
            l_prot_id   INTEGER,
            l_domain    TEXT NOT NULL DEFAULT '',
            CONSTRAINT links_fk_1 FOREIGN KEY (l_p_id) REFERENCES processes(p_id),
            CONSTRAINT links_fk_1 FOREIGN KEY (l_prot_id) REFERENCES protocols(prot_id)
        );",
        NO_PARAMS,
    )?;

    Ok(())
}

fn update_db(filename: String, procs: Vec<Process>) -> Result<()> {
    Ok(())
}
