use crate::util::{StatefulList, TabsState};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::canvas::{Canvas, Line, Map, MapResolution, Rectangle},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List, ListItem,
        Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame,
};
use crate::{PROCESSES, LOGS};
use crate::net::Process;

pub struct App<'a> {
    pub title: &'a str,
    pub should_quit: bool,
    pub tabs: TabsState<'a>,
    pub show_logs: bool,
    pub show_help: bool,
    pub procs: StatefulList<Process>,
    pub logs: StatefulList<String>,
    pub help: StatefulList<String>,
    pub enhanced_graphics: bool,
}

impl<'a> App<'a> {
    pub fn new(title: &'a str, enhanced_graphics: bool) -> App<'a> {
        App {
            title,
            should_quit: false,
            tabs: TabsState::new(vec!["Tab0", "Tab1", "Tab2"]),
            show_logs: true,
            show_help: false,
            procs: StatefulList::with_items(PROCESSES.lock().unwrap().to_vec()),
            logs: StatefulList::with_items(LOGS.lock().unwrap().to_vec()),
            help: StatefulList::with_items(vec![
                String::from("h: display/hide help"),
                String::from("l: display/hide logs"),
                String::from("q: quit"),
            ]),
            enhanced_graphics,
        }
    }

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
            'l' => {
                self.show_logs = !self.show_logs;
            }
            'h' => {
                self.show_help = !self.show_help;
            }
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {
        self.procs = StatefulList::with_items(PROCESSES.lock().unwrap().to_vec());
        self.logs = StatefulList::with_items(LOGS.lock().unwrap().to_vec());
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
     * Draw tabs.
     */
    let titles = app
        .tabs
        .titles
        .iter()
        .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Green))))
        .collect();

    // TODO: get interfaces
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(app.title))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.tabs.index);
    f.render_widget(tabs, zones[0]);

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

    /*
     * Draw process list.
     */
    let style = Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD);

    let procs: Vec<ListItem> = app
        .procs
        .items
        .iter()
        .flat_map(|p| {
            let mut tmp = vec![ ListItem::new( Span::styled(p.to_string(), style)) ];
            let mut tlinks = p.get_tlinks().iter().map(|t| ListItem::new(t.to_string())).collect();
            let mut ulinks = p.get_ulinks().iter().map(|u| ListItem::new(u.to_string())).collect();

            tmp.append(&mut tlinks);
            tmp.append(&mut ulinks);
            tmp
        })
        .collect::<Vec<ListItem>>();

    let procs = List::new(procs)
        .block(Block::default().borders(Borders::ALL).title(" Processes "));

    f.render_stateful_widget(procs, central_zones[0], &mut app.procs.state);

    /*
     * Create the optional central right block layout.
     */
    if app.show_logs || app.show_help {
        let constraints = if app.show_logs && app.show_help {
            vec![Constraint::Percentage(50), Constraint::Percentage(50)]
        } else {
            vec![Constraint::Percentage(100)]
        };
        let central_right_zones = Layout::default()
            .constraints(constraints)
            .direction(Direction::Vertical)
            .split(central_zones[1]);

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

            f.render_widget(logs, central_right_zones[0]);
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

            let id = central_right_zones.len() - 1;
            f.render_widget(help, central_right_zones[id]);
        }
    }

    /*
     * Draw the summary.
     */
    let summary = Block::default()
         .title(" Summary ")
         .borders(Borders::ALL);
    f.render_widget(summary, zones[2]);
}
