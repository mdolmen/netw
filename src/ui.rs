use crate::util::{StatefulList, TabsState};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Tabs},
    Frame,
};
use crate::{PROCESSES, LOGS, DATES};
use crate::net::Process;
use crate::database::get_procs;

use rusqlite::Connection;

pub struct App<'a> {
    pub title: &'a str,
    pub should_quit: bool,
    pub tabs: TabsState,
    pub show_logs: bool,
    pub show_help: bool,
    pub show_tcp: bool,
    pub show_udp: bool,
    pub show_all: bool,
    pub procs: StatefulList<Process>,
    pub logs: StatefulList<String>,
    pub help: StatefulList<String>,
    pub enhanced_graphics: bool,
    pub db: Option<Connection>,
}

impl<'a> App<'a> {
    pub fn new(title: &'a str, enhanced_graphics: bool) -> App<'a> {
        App {
            title,
            should_quit: false,
            tabs: TabsState::new(DATES.lock().unwrap().to_vec()),
            show_logs: true,
            show_help: true,
            show_tcp: false,
            show_udp: false,
            show_all: false,
            procs: StatefulList::new(),
            logs: StatefulList::with_items(LOGS.lock().unwrap().to_vec()),
            help: StatefulList::with_items(vec![
                String::from("H: display/hide help"),
                String::from("L: display/hide logs"),
                String::from("t: display/hide TCP"),
                String::from("u: display/hide UDP"),
                String::from("a: display/hide all (TCP+UDP)"),
                String::from(""),
                String::from("Arrows or hjkl: move around (main pane and tabs)"),
                String::from("q: quit"),
            ]),
            enhanced_graphics,
            db: None,
        }
    }

    pub fn procs(&mut self, procs: Vec<Process>) -> &mut Self {
        self.procs = StatefulList::with_items(procs);
        self
    }

    pub fn db(&mut self, db: Connection) -> &mut Self {
        self.db = Some(db);
        self
    }

    // TODO: scroll the process list
    pub fn on_up(&mut self) {
        self.procs.previous();
    }

    pub fn on_down(&mut self) {
        self.procs.next();
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_left(&mut self) {
        self.tabs.previous();
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            'h' => {
                self.tabs.previous();
            }
            'j' => {
                self.procs.next();
            }
            'k' => {
                self.procs.previous();
            }
            'l' => {
                self.tabs.next();
            }
            'L' => {
                self.show_logs = !self.show_logs;
            }
            'H' => {
                self.show_help = !self.show_help;
            }
            't' => {
                self.show_tcp = !self.show_tcp;
            }
            'u' => {
                self.show_udp = !self.show_udp;
            }
            'a' => {
                self.show_all = !self.show_all;
            }
            // TODO
            // 'v' for verbose
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {
        match self.db {
            Some(_) => {
                let db = self.db.as_ref().unwrap();
                self.procs = StatefulList::with_items(get_procs(&db));
            }
            None    => {
                self.procs = StatefulList::with_items(PROCESSES.lock().unwrap().to_vec());
            }
        }
        self.logs = StatefulList::with_items(LOGS.lock().unwrap().to_vec());
        self.tabs = TabsState::new(DATES.lock().unwrap().to_vec());
    }
}

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    /*
     * Create the main layout: split the screen in 3 blocks.
     */
    let zones = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Max(f.size().height),
                Constraint::Length(3)
            ].as_ref()
        )
        .split(f.size());

    /*
     * Create the layout for the central zone. Either one big window or 2 horizontal ones if the
     * user wants to show the logs.
     */
    let constraints = if app.show_logs || app.show_help {
        vec![Constraint::Percentage(65), Constraint::Percentage(35)]
    } else {
        vec![Constraint::Percentage(100)]
    };
    let central_zones = Layout::default()
        .constraints(constraints)
        .direction(Direction::Horizontal)
        .split(zones[1]);

    draw_tabs(f, app, zones[0]);
    draw_procs(f, app, central_zones[0]);
    draw_optionals(f, app, central_zones[1]);
    draw_filter(f, app, zones[2]);
}

pub fn draw_tabs<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let titles: Vec<String> = app.tabs.titles.iter().map( |t| t.str_form.to_string() ).collect();
    let titles = titles
        .iter()
        .map(|t| Spans::from(Span::styled(t, Style::default().fg(Color::White))))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(app.title))
        .highlight_style(Style::default().fg(Color::Green))
        .select(app.tabs.index);
    f.render_widget(tabs, area);
}

pub fn draw_procs<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let style0 = Style::default().add_modifier(Modifier::BOLD);
    let style1 = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);

    let mut date = 0;
    if app.tabs.titles.len() > 0 {
        date = app.tabs.titles[app.tabs.index].int_form;
    }

    let entries: Vec<ListItem> = app
        .procs
        .items
        .iter()
        .filter(|p| {
            p.date == date
        })
        .flat_map(|p| {
            let proc_fmt = Spans::from(vec![
                Span::styled(p.overview_str(), style0),
                Span::styled(p.data_amount_str(), style1),
            ]);

            let mut tmp = vec![ ListItem::new(proc_fmt) ];

            if app.show_tcp || app.show_all {
                let mut tlinks = p.get_tlinks()
                    .iter()
                    .map(|t| ListItem::new(t.to_string())
                ).collect();
                tmp.append(&mut tlinks);
            }

            if app.show_udp || app.show_all {
                let mut ulinks = p.get_ulinks()
                    .iter()
                    .map(|u| ListItem::new(u.to_string())
                ).collect();
                tmp.append(&mut ulinks);
            }
            tmp
        })
        .collect::<Vec<ListItem>>();

    // Number of line to display, not just nb of processes
    app.procs.nb_entries = entries.len();

    let entries = List::new(entries)
        .block(Block::default().borders(Borders::ALL).title(" Processes "))
        .highlight_style(Style::default().fg(Color::Green));

    f.render_stateful_widget(entries, area, &mut app.procs.state);
}

pub fn draw_optionals<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    if app.show_logs || app.show_help {
        // TODO: make it a function
        let constraints = if app.show_logs && app.show_help {
            vec![Constraint::Percentage(50), Constraint::Percentage(50)]
        } else {
            vec![Constraint::Percentage(100)]
        };
        let panes = Layout::default()
            .constraints(constraints)
            .direction(Direction::Vertical)
            .split(area);

        /*
         * Draw logs.
         */
        if app.show_logs {
            let logs: Vec<ListItem> = app
                .logs
                .items
                .iter()
                .map(|i| ListItem::new(Span::raw(i)))
                .collect();

            let logs = List::new(logs)
                .block(Block::default().borders(Borders::ALL).title(" Logs "))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            f.render_widget(logs, panes[0]);
        }

        /*
         * Draw help.
         */
        if app.show_help {
            let help: Vec<ListItem> = app
                .help
                .items
                .iter()
                .map(|i| ListItem::new(Span::raw(i)))
                .collect();

            let help = List::new(help)
                .block(Block::default().borders(Borders::ALL).title(" Help "))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            let id = panes.len() - 1;
            f.render_widget(help, panes[id]);
        }
    }
}

pub fn draw_filter<B: Backend>(f: &mut Frame<B>, _app: &mut App, area: Rect) {
    let filter = Block::default()
         .title(" Filter (TODO) ")
         .borders(Borders::ALL);
    f.render_widget(filter, area);
}
