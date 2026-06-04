use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone)]
pub struct FillerPhrase {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub text: String,
    pub audio: Vec<u8>,
}

#[derive(Clone)]
pub struct FillerLibrary {
    fillers: HashMap<String, FillerPhrase>,
}

impl FillerLibrary {
    pub fn load() -> Self {
        let mut fillers = HashMap::new();

        let definitions = [
            ("one_moment", "One moment."),
            ("let_me_check", "Let me check."),
            ("checking_now", "Checking now."),
            ("sure_thing", "Sure thing."),
            ("right_away", "Right away."),
        ];

        let sounds_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("sounds");

        for (name, text) in &definitions {
            let path = sounds_dir.join(format!("{}.mp3", name));
            if let Ok(audio) = std::fs::read(&path) {
                fillers.insert(name.to_string(), FillerPhrase {
                    name: name.to_string(),
                    text: text.to_string(),
                    audio,
                });
            }
        }

        if !fillers.is_empty() {
            eprintln!("notclicky: loaded {} filler phrases", fillers.len());
        }

        Self { fillers }
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&FillerPhrase> {
        self.fillers.get(name)
    }

    pub fn default_filler(&self) -> Option<&FillerPhrase> {
        self.fillers.get("one_moment")
    }

    pub fn pick_for_transcript(&self, transcript: &str) -> Option<&FillerPhrase> {
        let lower = transcript.to_lowercase();
        let screen_words = ["screen", "click", "this", "that", "here", "window", "button", "menu"];
        let is_screen_related = screen_words.iter().any(|w| lower.contains(w));

        if is_screen_related {
            self.fillers.get("checking_now").or_else(|| self.default_filler())
        } else {
            self.default_filler()
        }
    }
}
