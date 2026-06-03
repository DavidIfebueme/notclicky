use gtk4::prelude::*;
use gtk4::{
    Box, Button, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, Separator,
    Stack, StackSwitcher, TextView, Window,
};
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::memory::conversation::ConversationHistory;
use crate::memory::wiki::WikiManager;
use crate::notclicky_app::NotClickyApp;

pub struct PanelState {
    history: ConversationHistory,
    conversations: Vec<SavedConversation>,
    current_conversation: usize,
    mini_mode: bool,
}

#[derive(Clone)]
struct SavedConversation {
    id: String,
    title: String,
    preview: String,
    history: ConversationHistory,
}

pub fn build_panel(app: &adw::Application, nc_app: &NotClickyApp) -> adw::ApplicationWindow {
    let state = Rc::new(RefCell::new(PanelState {
        history: ConversationHistory::new(),
        conversations: vec![SavedConversation {
            id: "new".to_string(),
            title: "New Chat".to_string(),
            preview: String::new(),
            history: ConversationHistory::new(),
        }],
        current_conversation: 0,
        mini_mode: false,
    }));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("NotClicky")
        .default_width(580)
        .default_height(640)
        .build();

    let main_layout = Box::new(Orientation::Horizontal, 0);

    let sidebar = build_conversation_sidebar(&state);
    let content_area = build_content_area(&state, nc_app);

    main_layout.append(&sidebar);
    main_layout.append(&gtk4::Separator::new(Orientation::Vertical));
    main_layout.append(&content_area);

    window.set_content(Some(&main_layout));
    window
}

fn build_conversation_sidebar(state: &Rc<RefCell<PanelState>>) -> gtk4::Widget {
    let sidebar = Box::new(Orientation::Vertical, 0);
    sidebar.set_width_request(180);

    let title = Label::new(Some("Conversations"));
    title.add_css_class("title-4");
    title.set_margin_start(8);
    title.set_margin_top(8);
    title.set_margin_bottom(4);

    let list = ListBox::new();
    list.add_css_class("navigation-sidebar");

    let row = ListBoxRow::new();
    row.set_child(Some(&Label::new(Some("New Chat"))));
    list.append(&row);

    let new_button = Button::with_label("+ New");
    new_button.add_css_class("flat");
    new_button.set_margin_start(8);
    new_button.set_margin_end(8);
    new_button.set_margin_bottom(8);

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&list));

    sidebar.append(&title);
    sidebar.append(&scrolled);
    sidebar.append(&new_button);

    sidebar.upcast()
}

fn build_content_area(state: &Rc<RefCell<PanelState>>, nc_app: &NotClickyApp) -> gtk4::Widget {
    let content = Box::new(Orientation::Vertical, 0);

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("NotClicky", "Your AI Companion"))
        .build();

    let memory_button = Button::with_label("Memory");
    memory_button.add_css_class("flat");

    let mini_button = Button::with_label("Mini");
    mini_button.add_css_class("flat");

    header.pack_start(&memory_button);
    header.pack_end(&mini_button);

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

    setup_autocomplete(&entry);

    let send_button = Button::with_label("Send");
    send_button.add_css_class("suggested-action");

    input_box.append(&entry);
    input_box.append(&send_button);
    content.append(&input_box);

    let llm = nc_app.llm.clone();
    let state_ref = state.clone();
    let messages_ref = messages_list.clone();
    entry.connect_activate(move |entry| {
        handle_send(entry, &messages_ref, &state_ref, &llm);
    });

    let llm = nc_app.llm.clone();
    let state_ref = state.clone();
    let messages_ref = messages_list.clone();
    let entry_clone = entry.clone();
    send_button.connect_clicked(move |_| {
        handle_send(&entry_clone, &messages_ref, &state_ref, &llm);
    });

    let window_ref: Rc<RefCell<Option<gtk4::Window>>> = Rc::new(RefCell::new(None));
    memory_button.connect_clicked(move |_| {
        show_memory_drawer(&window_ref);
    });

    content.upcast()
}

fn setup_autocomplete(entry: &Entry) {
    let completions = [
        "agent ", "screenshot", "point to ", "highlight ", "speak ",
        "clear overlay", "search wiki", "remember ", "what is ",
        "help me ", "build ", "fix ", "explain ",
    ];

    entry.connect_changed(move |entry| {
        let text = entry.text().to_string();
        if text.is_empty() {
            return;
        }

        let lower = text.to_lowercase();
        for comp in &completions {
            if comp.starts_with(&lower) && comp.len() > lower.len() {
                let suffix = &comp[lower.len()..];
                let pos = entry.position();
                entry.set_text(comp);
                entry.set_position(pos as i32);
                entry.select_region(pos as i32, comp.len() as i32);
                break;
            }
        }
    });
}

fn handle_send(
    entry: &Entry,
    messages_list: &ListBox,
    state: &Rc<RefCell<PanelState>>,
    llm: &std::sync::Arc<tokio::sync::Mutex<std::boxed::Box<dyn crate::ai::providers::LlmProvider>>>,
) {
    let text = entry.text().to_string();
    if text.trim().is_empty() {
        return;
    }
    entry.set_text("");

    add_message(messages_list, &text, "user");

    let (response_tx, response_rx) = std::sync::mpsc::channel::<String>();
    let messages_for_response = messages_list.clone();

    gtk4::glib::idle_add_local(move || {
        match response_rx.try_recv() {
            Ok(response) => {
                add_message(&messages_for_response, &response, "assistant");
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return gtk4::glib::ControlFlow::Break;
            }
            Err(_) => {}
        }
        gtk4::glib::ControlFlow::Continue
    });

    let llm_clone = llm.clone();
    let user_text = text.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("llm runtime");
        let response = rt.block_on(async {
            let req = crate::ai::providers::LlmRequest {
                messages: vec![
                    crate::ai::providers::LlmMessage {
                        role: "system".to_string(),
                        content: "You are NotClicky, a helpful Linux desktop companion. Be concise and direct.".to_string(),
                    },
                    crate::ai::providers::LlmMessage {
                        role: "user".to_string(),
                        content: user_text,
                    },
                ],
                model: None,
                max_tokens: None,
                temperature: None,
            };

            let provider = llm_clone.lock().await;
            match provider.stream(req).await {
                Ok(mut stream) => {
                    let mut response = String::new();
                    loop {
                        match futures::StreamExt::next(&mut stream).await {
                            Some(Ok(token)) => response.push_str(&token),
                            Some(Err(_)) => break,
                            None => break,
                        }
                    }
                    if response.is_empty() { "No response.".to_string() } else { response }
                }
                Err(e) => format!("Error: {}", e),
            }
        });
        let _ = response_tx.send(response);
    });

    let mut state = state.borrow_mut();
    state.history.add(text, String::new());
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

fn show_memory_drawer(window_ref: &Rc<RefCell<Option<gtk4::Window>>>) {
    let mut store = window_ref.borrow_mut();
    if store.is_some() {
        if let Some(ref win) = *store {
            win.present();
        }
        return;
    }

    let window = Window::new();
    window.set_title(Some("NotClicky — Memory & Wiki"));
    window.set_default_size(480, 520);

    let content = Box::new(Orientation::Vertical, 0);

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("Memory", "Wiki and persistent memory"))
        .build();
    content.append(&header);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let memory_box = Box::new(Orientation::Vertical, 8);
    memory_box.set_margin_start(12);
    memory_box.set_margin_end(12);
    memory_box.set_margin_top(8);

    let memory_label = Label::new(Some("Persistent Memory"));
    memory_label.add_css_class("title-4");
    memory_box.append(&memory_label);

    let memory_view = TextView::new();
    memory_view.set_vexpand(true);
    memory_view.set_wrap_mode(gtk4::WrapMode::WordChar);

    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    let memory_path = config_dir.join("notclicky").join("memory.md");
    let mut memory = crate::memory::conversation::PersistentMemory::new(memory_path);
    let buffer = memory_view.buffer();
    buffer.set_text(memory.get());

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&memory_view));
    memory_box.append(&scrolled);

    let save_button = Button::with_label("Save Memory");
    save_button.add_css_class("suggested-action");
    save_button.set_halign(gtk4::Align::End);
    memory_box.append(&save_button);

    stack.add_titled(&memory_box, Some("memory"), "Memory");

    let wiki_box = Box::new(Orientation::Vertical, 8);
    wiki_box.set_margin_start(12);
    wiki_box.set_margin_end(12);
    wiki_box.set_margin_top(8);

    let wiki_label = Label::new(Some("Wiki"));
    wiki_label.add_css_class("title-4");
    wiki_box.append(&wiki_label);

    let wiki_list = ListBox::new();
    wiki_list.add_css_class("boxed-list");

    let resources_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let wiki_dir = config_dir.join("notclicky").join("wiki");
    if !wiki_dir.exists() {
        let _ = std::fs::create_dir_all(&wiki_dir);
        let seed_dir = resources_dir.join("wiki");
        if seed_dir.exists() {
            let mut wm = WikiManager::new(wiki_dir.clone());
            let _ = wm.import_seed(&seed_dir);
        }
    }

    let mut wm = WikiManager::new(wiki_dir);
    let _ = wm.load();
    for page in wm.list() {
        let row = ListBoxRow::new();
        row.set_child(Some(&Label::new(Some(&format!("{} — {}", page.title, truncate(&page.content, 60))))));
        wiki_list.append(&row);
    }

    let wiki_scrolled = ScrolledWindow::new();
    wiki_scrolled.set_vexpand(true);
    wiki_scrolled.set_child(Some(&wiki_list));
    wiki_box.append(&wiki_scrolled);

    stack.add_titled(&wiki_box, Some("wiki"), "Wiki");

    let switcher = StackSwitcher::new();
    switcher.set_stack(Some(&stack));

    content.append(&switcher);
    content.append(&stack);

    window.set_child(Some(&content));
    window.present();

    *store = Some(window);
}

pub fn build_mini_panel(app: &adw::Application) -> adw::ApplicationWindow {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("NotClicky Mini")
        .default_width(320)
        .default_height(400)
        .build();

    let content = Box::new(Orientation::Vertical, 0);

    let header = adw::HeaderBar::builder()
        .title_widget(&adw::WindowTitle::new("NotClicky", ""))
        .build();

    content.append(&header);

    let messages_list = ListBox::new();
    messages_list.add_css_class("rich-list");
    messages_list.set_selection_mode(gtk4::SelectionMode::None);

    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_child(Some(&messages_list));

    content.append(&scrolled);

    let entry = Entry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Ask..."));
    entry.set_margin_start(6);
    entry.set_margin_end(6);
    entry.set_margin_top(4);
    entry.set_margin_bottom(4);

    content.append(&entry);

    window.set_content(Some(&content));
    window
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
