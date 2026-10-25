use gpui_component::input::{InputEvent, InputState};

use crate::*;

impl Gallery {
    pub(crate) fn new_with_mode(
        init_story: Option<&str>,
        embedded: bool,
        hide_status_bar: bool,
        hide_sidebar: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search…"));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change => {
                this.active_group_index = Some(0);
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let stories = vec![(
            "",
            vec![
                StoryContainer::panel::<WelcomeStory>(window, cx),
                StoryContainer::panel::<AccordionStory>(window, cx),
                StoryContainer::panel::<AlertStory>(window, cx),
                StoryContainer::panel::<AlertDialogStory>(window, cx),
                StoryContainer::panel::<AvatarStory>(window, cx),
                StoryContainer::panel::<BadgeStory>(window, cx),
                StoryContainer::panel::<BreadcrumbStory>(window, cx),
                StoryContainer::panel::<ButtonStory>(window, cx),
                StoryContainer::panel::<CalendarStory>(window, cx),
                StoryContainer::panel::<ChartStory>(window, cx),
                StoryContainer::panel::<CheckboxStory>(window, cx),
                StoryContainer::panel::<ClipboardStory>(window, cx),
                StoryContainer::panel::<CollapsibleStory>(window, cx),
                StoryContainer::panel::<ColorPickerStory>(window, cx),
                StoryContainer::panel::<ComboboxStory>(window, cx),
                StoryContainer::panel::<CommandStory>(window, cx),
                StoryContainer::panel::<DataTableStory>(window, cx),
                StoryContainer::panel::<DatePickerStory>(window, cx),
                StoryContainer::panel::<DescriptionListStory>(window, cx),
                StoryContainer::panel::<DialogStory>(window, cx),
                StoryContainer::panel::<DropdownButtonStory>(window, cx),
                StoryContainer::panel::<EditorStory>(window, cx),
                StoryContainer::panel::<FormStory>(window, cx),
                StoryContainer::panel::<GroupBoxStory>(window, cx),
                StoryContainer::panel::<HoverCardStory>(window, cx),
                StoryContainer::panel::<IconStory>(window, cx),
                StoryContainer::panel::<ImageStory>(window, cx),
                StoryContainer::panel::<InputStory>(window, cx),
                StoryContainer::panel::<KbdStory>(window, cx),
                StoryContainer::panel::<LabelStory>(window, cx),
                StoryContainer::panel::<ListStory>(window, cx),
                StoryContainer::panel::<MenuStory>(window, cx),
                StoryContainer::panel::<NativeMenuStory>(window, cx),
                StoryContainer::panel::<NotificationStory>(window, cx),
                StoryContainer::panel::<NumberInputStory>(window, cx),
                StoryContainer::panel::<OtpInputStory>(window, cx),
                StoryContainer::panel::<PaginationStory>(window, cx),
                StoryContainer::panel::<PopoverStory>(window, cx),
                StoryContainer::panel::<ProgressStory>(window, cx),
                StoryContainer::panel::<RadioStory>(window, cx),
                StoryContainer::panel::<RatingStory>(window, cx),
                StoryContainer::panel::<ResizableStory>(window, cx),
                StoryContainer::panel::<ScrollbarStory>(window, cx),
                StoryContainer::panel::<SelectStory>(window, cx),
                StoryContainer::panel::<SeparatorStory>(window, cx),
                StoryContainer::panel::<SettingsStory>(window, cx),
                StoryContainer::panel::<SheetStory>(window, cx),
                StoryContainer::panel::<SidebarStory>(window, cx),
                StoryContainer::panel::<SkeletonStory>(window, cx),
                StoryContainer::panel::<SliderStory>(window, cx),
                StoryContainer::panel::<SpinnerStory>(window, cx),
                StoryContainer::panel::<StatusBarStory>(window, cx),
                StoryContainer::panel::<StepperStory>(window, cx),
                StoryContainer::panel::<SwitchStory>(window, cx),
                StoryContainer::panel::<TableStory>(window, cx),
                StoryContainer::panel::<TabsStory>(window, cx),
                StoryContainer::panel::<TagStory>(window, cx),
                StoryContainer::panel::<TextareaStory>(window, cx),
                StoryContainer::panel::<ThemeColorsStory>(window, cx),
                StoryContainer::panel::<ToggleStory>(window, cx),
                StoryContainer::panel::<TooltipStory>(window, cx),
                StoryContainer::panel::<TreeStory>(window, cx),
                StoryContainer::panel::<VirtualListStory>(window, cx),
            ],
        )];

        let mut this = Self {
            search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            collapsed: false,
            embedded,
            hide_status_bar,
            hide_sidebar,
            _subscriptions,
        };

        if let Some(init_story) = init_story {
            this.set_active_story(init_story, window, cx);
        }

        this
    }
}
