pub trait ContainsOSM {
    fn contains_art(&self) -> bool;
    fn contains_link(&self) -> bool;
    fn contains_name(&self) -> bool;
    fn contains_other(&self) -> bool;
    fn contains_adr(&self) -> bool;
    fn contains_museum(&self) -> bool;
}

impl ContainsOSM for String {
    fn contains_art(&self) -> bool {
        let lowercase = self.to_lowercase();
        let mut words = lowercase.split(" ").map(|v| v.trim());

        words.any(|s| {
            s == "art"
                || s == "изкуство"
                || s == "kunst"
                || s == "taide"
                || s == "τέχνη"
                || s == "Ealaín"
                || s == "gr"
                || s == "arte"
                || s == "קונסט"
                || s == "umjetnost"
                || s == "чл"
                || s == "sztuka"
                || s == "artă"
                || s == "искусство"
                || s == "konst"
                || s == "уметност"
                || s == "čl"
                || s == "Umetnost"
                || s == "umění"
                || s == "ст"
                || s == "művészet"
                || s == "celf"
                || s == "арт"
        })
    }

    fn contains_link(&self) -> bool {
        (self.contains("website")
            || self.contains("http")
            || self.contains("https")
            || self.contains("www.")
            || self.contains(".com")
            || self.contains(".de")
            || self.contains(".at")
            || self.contains(".uk")
            || self.contains(".eu")
            || self.contains(".it")
            || self.contains(".by")
            || self.contains(".ch")
            || self.contains(".cz")
            || self.contains(".am")
            || self.contains(".bg")
            || self.contains(".dk")
            || self.contains(".ee")
            || self.contains(".es")
            || self.contains(".fi")
            || self.contains(".fr")
            || self.contains(".gl")
            || self.contains(".gr")
            || self.contains(".hr")
            || self.contains(".hu")
            || self.contains(".ie")
            || self.contains(".is")
            || self.contains(".je")
            || self.contains(".li")
            || self.contains(".lt")
            || self.contains(".lu")
            || self.contains(".lv")
            || self.contains(".mc")
            || self.contains(".md")
            || self.contains(".me")
            || self.contains(".nl")
            || self.contains(".no")
            || self.contains(".pl")
            || self.contains(".pt")
            || self.contains(".ro")
            || self.contains(".rs")
            || self.contains(".se")
            || self.contains(".si")
            || self.contains(".sk")
            || self.contains(".ua"))
            && !self.contains("@")
    }

    fn contains_name(&self) -> bool {
        self.contains("name")
    }

    fn contains_other(&self) -> bool {
        self.contains("contact")
    }

    fn contains_adr(&self) -> bool {
        self.contains("city") || self.contains("country")
    }

    fn contains_museum(&self) -> bool {
        self.contains("museum")
    }
}
