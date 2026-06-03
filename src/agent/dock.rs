use gtk4::prelude::*;
use gtk4::{Box, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Spinner};

use crate::agent::session::{AgentSession, AgentStatus};

#[allow(dead_code)]
pub struct AgentDock {
    container: Box,
    list: ListBox,
    rows: std::cell::RefCell<Vec<(String, ListBoxRow, Label, Spinner)>>,
}

impl AgentDock {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let container = Box::new(Orientation::Vertical, 8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);

        let title = Label::new(Some("Agents"));
        title.add_css_class("title-4");
        container.append(&title);

        let list = ListBox::new();
        list.add_css_class("boxed-list");

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&list));
        container.append(&scrolled);

        Self {
            container,
            list,
            rows: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[allow(dead_code)]
    pub fn widget(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }

    #[allow(dead_code)]
    pub fn add_session(&self, session: &AgentSession) {
        let row = ListBoxRow::new();
        let hbox = Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(4);
        hbox.set_margin_bottom(4);

        let spinner = Spinner::new();
        if session.status != AgentStatus::Running && session.status != AgentStatus::Starting {
            spinner.set_spinning(false);
        }

        let status_icon = match session.status {
            AgentStatus::Starting | AgentStatus::Running => "⏳",
            AgentStatus::Done => "✅",
            AgentStatus::Failed => "❌",
        };

        let label = Label::new(Some(&format!(
            "{} {} — {}",
            status_icon,
            &session.id[..16.min(session.id.len())],
            truncate(&session.prompt, 40)
        )));
        label.set_hexpand(true);
        label.set_halign(gtk4::Align::Start);

        hbox.append(&spinner);
        hbox.append(&label);
        row.set_child(Some(&hbox));

        self.list.append(&row);
        self.rows.borrow_mut().push((session.id.clone(), row, label, spinner));
    }

    #[allow(dead_code)]
    pub fn update_session(&self, session: &AgentSession) {
        let mut rows = self.rows.borrow_mut();
        if let Some(entry) = rows.iter_mut().find(|(id, _, _, _)| id == &session.id) {
            let status_icon = match session.status {
                AgentStatus::Starting | AgentStatus::Running => "⏳",
                AgentStatus::Done => "✅",
                AgentStatus::Failed => "❌",
            };
            entry.2.set_label(&format!(
                "{} {} — {}",
                status_icon,
                &session.id[..16.min(session.id.len())],
                truncate(&session.prompt, 40)
            ));

            let spinning = matches!(session.status, AgentStatus::Starting | AgentStatus::Running);
            entry.3.set_spinning(spinning);
        }
    }

    #[allow(dead_code)]
    pub fn remove_session(&self, id: &str) {
        let mut rows = self.rows.borrow_mut();
        if let Some(pos) = rows.iter().position(|(sid, _, _, _)| sid == id) {
            let (_, row, _, _) = rows.remove(pos);
            self.list.remove(&row);
        }
    }
}

#[allow(dead_code)]
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
