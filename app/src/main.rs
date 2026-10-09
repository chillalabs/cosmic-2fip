mod about;
mod app;
mod connect;
mod connections_panel;
mod context_menu;
mod dnd;
mod file_item;
mod find;
mod icon_sets;
mod keybinds;
mod launch;
mod localize;
mod menu_bar;
mod operation;
mod pane;
mod tab;
mod themes;
#[cfg(windows)]
mod windows;

fn main() -> cosmic::iced::Result {
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
