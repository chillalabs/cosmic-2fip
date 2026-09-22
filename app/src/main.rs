use cosmic::app::{Core, Task};
use cosmic::widget;
use cosmic::{Application, Element};

#[derive(Default)]
struct App {
    core: Core,
}

#[derive(Debug, Clone)]
enum Message {}

impl Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "dev.gonzalo.CosmicCommander";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        (App { core }, Task::none())
    }

    fn update(&mut self, _message: Self::Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        widget::text("cosmic-commander").into()
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::app::run::<App>(cosmic::app::Settings::default(), ())
}
