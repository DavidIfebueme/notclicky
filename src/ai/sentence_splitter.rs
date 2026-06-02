pub struct SentenceSplitter {
    buffer: String,
}

impl SentenceSplitter {
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    pub fn push(&mut self, token: &str) -> Vec<String> {
        self.buffer.push_str(token);
        let mut sentences = Vec::new();

        while let Some(pos) = find_sentence_end(&self.buffer) {
            let sentence: String = self.buffer[..pos].trim().to_string();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            self.buffer = self.buffer[pos..].to_string();
        }

        sentences
    }

    pub fn flush(&mut self) -> Option<String> {
        let remaining = self.buffer.trim().to_string();
        self.buffer.clear();
        if remaining.is_empty() {
            None
        } else {
            Some(remaining)
        }
    }
}

fn find_sentence_end(text: &str) -> Option<usize> {
    let endings = [". ", "! ", "? ", ".\n", "!\n", "?\n"];
    for ending in &endings {
        if let Some(pos) = text.find(ending) {
            return Some(pos + ending.len());
        }
    }
    if text.ends_with('.') || text.ends_with('!') || text.ends_with('?') {
        return Some(text.len());
    }
    None
}
