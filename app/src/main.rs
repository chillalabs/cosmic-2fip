mod app;
mod context_menu;
mod file_item;
mod keybinds;
mod launch;
mod localize;
mod menu_bar;
mod operation;
mod pane;
mod tab;

fn main() -> cosmic::iced::Result {
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
