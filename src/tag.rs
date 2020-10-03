use xcb::Window;

pub struct Tag {
    name: String,
    windows: Vec<Window>,
}

impl Tag {
    pub fn new(name: impl Into<String>) -> Tag {
        Tag {
            name: name.into(),
            windows: Vec::new(),
        }
    }

    pub fn add_window(&mut self, win: Window) {
        self.windows.push(win);
    }

    pub fn remove_window(&mut self, win: Window) {
        self.windows.swap_remove(self.windows.iter().position(|x| *x == win).expect("window not found in tag"));
    }

    pub fn windows(&self) -> &Vec<Window> {
        &self.windows
    }
}
