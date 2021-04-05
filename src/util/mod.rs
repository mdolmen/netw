pub mod event;

use tui::widgets::ListState;

/*
 * Save the selected item index to keep the selection across UI refresh.
 */
static mut SELECTED: usize = 0;

pub struct TabsState<> {
    pub titles: Vec<String>,
    pub index: usize,
}

impl TabsState {
    pub fn new(titles: Vec<String>) -> TabsState {
        TabsState { titles, index: 0 }
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
    pub nb_items: usize,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
            nb_items: 0,
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        // Unsafe because of access to global mut var
        unsafe { 
            let mut state =  ListState::default();
            state.select(Some(SELECTED));

            StatefulList {
                state,
                items,
                nb_items: 0,
            }
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.nb_items - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        unsafe { SELECTED = i; }
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.nb_items - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        unsafe { SELECTED = i; }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}
