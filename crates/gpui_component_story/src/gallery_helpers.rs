use gpui_component::command::CommandEntry;

fn component_command(name: impl Into<SharedString>) -> CommandEntry {
    CommandItem::new().label(name).into()
}

fn find_story_index<'a>(
    groups: impl IntoIterator<Item = impl IntoIterator<Item = &'a str>>,
    name: &str,
) -> Option<(usize, usize)> {
    groups
        .into_iter()
        .enumerate()
        .find_map(|(group_ix, group)| {
            group
                .into_iter()
                .position(|story_name| story_name.eq_ignore_ascii_case(name))
                .map(|story_ix| (group_ix, story_ix))
        })
}