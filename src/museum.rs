pub struct Museum {
    pub names: Vec<String>,
    pub adr: Vec<String>,
    pub other: Vec<String>,
}

impl Museum {
    pub fn add_name(&mut self, new: String) {
        self.names.push(new)
    }

    pub fn add_adr(&mut self, new: String) {
        self.adr.push(new)
    }

    pub fn add_other(&mut self, new: String) {
        self.other.push(new)
    }
}

impl Default for Museum {
    fn default() -> Self {
        Self {
            names: Vec::new(),
            adr: Vec::new(),
            other: Vec::new(),
        }
    }
}

impl ToString for Museum {
    fn to_string(&self) -> String {
        format!(
            "Name: {:?}\nAdresse: {:?}\nAnderes: {:?}\n\n",
            self.names, self.adr, self.other
        )
    }
}
