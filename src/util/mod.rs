pub mod event;

use tui::widgets::ListState;
use crate::Date;

/*
 * Save the selected item index to keep the selection across UI refresh.
 */
static mut SELECTED_ENTRY: usize = 0;
static mut SELECTED_TAB: usize = 0;

pub struct TabsState<> {
    pub titles: Vec<Date>,
    pub index: usize,
}

impl TabsState {
    pub fn new(titles: Vec<Date>) -> TabsState {
        unsafe {
            TabsState {
                titles,
                index: SELECTED_TAB,
            }
        }
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
        unsafe { SELECTED_TAB = self.index; }
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
        unsafe { SELECTED_TAB = self.index; }
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    pub nb_entries: usize,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
            nb_entries: 0,
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        // Unsafe because of access to global mut var
        unsafe {
            let mut state =  ListState::default();
            state.select(Some(SELECTED_ENTRY));

            StatefulList {
                state,
                items,
                nb_entries: 0,
            }
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.nb_entries - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        unsafe { SELECTED_ENTRY = i; }
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.nb_entries - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        unsafe { SELECTED_ENTRY = i; }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
