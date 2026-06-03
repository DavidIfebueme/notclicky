use gtk4::prelude::*;
use gtk4::{Box, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Separator};
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::memory::conversation::ConversationHistory;

pub struct PanelState {
    history: ConversationHistory,
}

pub fn build_panel(app: &adw::Application) -> adw::ApplicationWindow {
    let state = Rc::new(RefCell::new(PanelState {
        history: ConversationHistory::new(),
    }));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("NotClicky")
        .default_width(420)
        .default_height(640)
        .build();

    let content = Box::new(Orientation::Vertical, 0);

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("NotClicky", "Your AI Companion"))
        .build();

    content.append(&header);
    content.append(&Separator::new(Orientation::Horizontal));

    let messages_list = ListBox::new();
    messages_list.add_css_class("rich-list");
    messages_list.set_selection_mode(gtk4::SelectionMode::None);

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&messages_list));

    content.append(&scrolled);
    content.append(&Separator::new(Orientation::Horizontal));

    let input_box = Box::new(Orientation::Horizontal, 6);
    input_box.set_margin_start(8);
    input_box.set_margin_end(8);
    input_box.set_margin_top(6);
    input_box.set_margin_bottom(6);

    let entry = Entry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Ask NotClicky anything..."));

    let send_button = Button::with_label("Send");
    send_button.add_css_class("suggested-action");

    input_box.append(&entry);
    input_box.append(&send_button);
    content.append(&input_box);

    let state_ref = state.clone();
    let messages_ref = messages_list.clone();
    entry.connect_activate(move |entry| {
        handle_send(entry, &messages_ref, &state_ref);
    });

    let state_ref = state.clone();
    let messages_ref = messages_list.clone();
    send_button.connect_clicked(move |_| {
        handle_send(&entry, &messages_ref, &state_ref);
    });

    window.set_content(Some(&content));
    window
}

fn handle_send(entry: &Entry, messages_list: &ListBox, state: &Rc<RefCell<PanelState>>) {
    let text = entry.text().to_string();
    if text.trim().is_empty() {
        return;
    }
    entry.set_text("");

    add_message(messages_list, &text, "user");

    let placeholder = "I'm NotClicky, your AI companion. Use voice mode (Ctrl+Alt) or type here to chat.".to_string();
    add_message(messages_list, &placeholder, "assistant");

    let mut state = state.borrow_mut();
    state.history.add(text, placeholder);
}

fn add_message(list: &ListBox, text: &str, role: &str) {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);

    let hbox = Box::new(Orientation::Horizontal, 8);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);

    let label = Label::new(Some(text));
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
    list.append(&row);
}
