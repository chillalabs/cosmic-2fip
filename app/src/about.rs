//! The About side panel (View → About 2fip): name, description, version,
//! repository and license, all taken from Cargo.toml.

use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

use crate::app::Message;
use crate::fl;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");

const ICON_SIZE: u16 = 96;

/// The About side panel's content. `app_id` names the app icon.
pub fn view(app_id: &'static str) -> Element<'static, Message> {
    let label = |text: String| widget::text::body(text).font(cosmic::font::bold());
    let details = widget::Column::new()
        .spacing(8)
        .align_x(Alignment::Center)
        .push(
            widget::Row::new()
                .spacing(6)
                .push(label(fl!("about-version")))
                .push(widget::text::body(VERSION)),
        )
        .push(
            widget::Row::new()
                .spacing(6)
                .align_y(Alignment::Center)
                .push(label(fl!("about-repository")))
                .push(widget::button::link(REPOSITORY).on_press(Message::OpenUrl(REPOSITORY))),
        )
        .push(
            widget::Row::new()
                .spacing(6)
                .push(label(fl!("about-license")))
                .push(widget::text::body(LICENSE)),
        );

    let content = widget::Column::new()
        .spacing(12)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .push(widget::icon::from_name(app_id).size(ICON_SIZE))
        .push(widget::text::title2("2fip"))
        .push(
            widget::text::body(fl!("about-description"))
                .align_x(Alignment::Center)
                .width(Length::Fill),
        )
        .push(widget::Space::new().height(4))
        .push(details);

    content.into()
}
