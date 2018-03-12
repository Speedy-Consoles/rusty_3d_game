pub struct Menu {
    active: bool,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            active: true,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn active(&self) -> bool {
        self.active
    }
}