use gtk4::prelude::*;
use gtk4::{Box, Label, ListBox, Orientation};
use libadwaita as adw;

pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct ChatWidget {
    messages_list: ListBox,
    messages: Vec<ChatMessage>,
}

impl ChatWidget {
    pub fn new() -> Self {
        let messages_list = ListBox::new();
        messages_list.add_css_class("rich-list");
        messages_list.set_selection_mode(gtk4::SelectionMode::None);

        Self {
            messages_list,
            messages: Vec::new(),
        }
    }

    pub fn widget(&self) -> &ListBox {
        &self.messages_list
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let row = gtk4::ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);

        let hbox = Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);

        let label = Label::new(Some(content));
        label.set_wrap(true);
        label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
        label.set_xalign(0.0);
        label.set_hexpand(true);

        if role == "user" {
            label.add_css_class("accent");
            hbox.set_halign(gtk4::Align::End);
        } else {
            hbox.set_halign(gtk4::Align::Start);
        }

        hbox.append(&label);
        row.set_child(Some(&hbox));
        self.messages_list.append(&row);

        self.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn update_last_message(&mut self, content: &str) {
        if let Some(last) = self.messages.last_mut() {
            last.content = content.to_string();
        }
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        while let Some(row) = self.messages_list.last_child() {
            self.messages_list.remove(&row);
        }
    }
}
