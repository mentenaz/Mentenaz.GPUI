use gpui::App;
use gpui_component::command::CommandEntry;

use crate::Gallery;

use super::component_command::component_command;

impl Gallery {
    pub(crate) fn command_entries(&self, cx: &App) -> Vec<CommandEntry> {
        self.stories
            .iter()
            .flat_map(|(_, stories)| stories)
            .map(|story| component_command(story.read(cx).name.clone()))
            .collect()
    }
}
