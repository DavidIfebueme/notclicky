use futures::Stream;

use crate::ai::sentence_splitter::SentenceSplitter;

pub struct SentenceStream {
    inner: crate::ai::providers::LlmStream,
    splitter: SentenceSplitter,
    pending: Vec<String>,
}

impl SentenceStream {
    pub fn new(inner: crate::ai::providers::LlmStream) -> Self {
        Self {
            inner,
            splitter: SentenceSplitter::new(),
            pending: Vec::new(),
        }
    }
}

impl Stream for SentenceStream {
    type Item = String;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        loop {
            if let Some(sentence) = self.pending.pop() {
                return std::task::Poll::Ready(Some(sentence));
            }

            match self.inner.as_mut().poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(token))) => {
                    let sentences = self.splitter.push(&token);
                    for s in sentences.into_iter().rev() {
                        self.pending.push(s);
                    }
                }
                std::task::Poll::Ready(Some(Err(_))) => continue,
                std::task::Poll::Ready(None) => {
                    if let Some(remaining) = self.splitter.flush() {
                        return std::task::Poll::Ready(Some(remaining));
                    }
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
    }
}
