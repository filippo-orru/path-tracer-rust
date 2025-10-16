use iced::{
    Color, Element, Length,
    widget::{column, combo_box, container, row, text, text_input},
};

use crate::{Message, State, viewport::ViewportProgram};

pub fn viewport_tab(state: &'_ State) -> Element<'_, Message> {
    row![ViewportProgram::view(&state.render_config).map(Message::ViewportMessage),]
        .spacing(10)
        .into()
}
