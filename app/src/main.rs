mod about;
mod app;
mod context_menu;
mod dnd;
mod file_item;
mod find;
mod keybinds;
mod launch;
mod localize;
mod menu_bar;
mod operation;
mod pane;
mod tab;
mod themes;

fn main() -> cosmic::iced::Result {
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
