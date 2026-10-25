use gpui::SharedString;
use gpui_component::command::{CommandEntry, CommandItem};

pub(crate) fn component_command(name: impl Into<SharedString>) -> CommandEntry {
    CommandItem::new().label(name).into()
}
