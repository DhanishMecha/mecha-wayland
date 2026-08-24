use renderer::commands::Color;
use taffy::prelude::*;
use taffy::{Size, Style};
use ui::widgets::{Div, Text};
use ui::{Damage, OnChange, Point, Render, RenderCommand};
use utils::Rect as UtilsRect;

use crate::atlas;
use crate::widgets::datetime::{DateTime, DateTimeUpdate};

pub type TimeUpdate = DateTimeUpdate;

fn header_style() -> Style {
    Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        justify_content: Some(JustifyContent::Start),
        align_items: Some(AlignItems::Start),
        gap: Size {
            width: zero(),
            height: length(8.0_f32),
        },
        ..Style::default()
    }
}

#[ui::widget]
pub struct TimeWidget {
    #[widget(child)]
    pub children: Div<(Text, Text)>,
}

impl Render for TimeWidget {
    fn render(&self, _layout: &taffy::Layout, _abs_pos: Point) -> Vec<RenderCommand> {
        Vec::new()
    }
}

impl TimeWidget {
    pub fn new() -> Self {
        let time_str = DateTime::format("%H:%M");
        let date_raw = DateTime::format("%a %d");
        let date_str = format!("{} · MECHA COMET", date_raw);
        Self::with_initial_time(time_str, date_str)
    }

    pub fn with_initial_time(time_str: String, date_str: String) -> Self {
        let mut time_widget = Text::new(Style::default());
        time_widget.font = Some(&atlas::LOCKSCREEN_FONT_GEIST_MONO_46);
        time_widget.color = Color::from_rgb8(242, 242, 240);
        time_widget.text = time_str.into();

        let mut date_widget = Text::new(Style::default());
        date_widget.font = Some(&atlas::LOCKSCREEN_FONT_GEIST_MONO_12);
        date_widget.color = Color::from_rgb8(255, 106, 31);
        date_widget.text = date_str.into();

        let header = Div::new(header_style(), (time_widget, date_widget));

        Self {
            node_id: taffy::NodeId::new(u64::MAX),
            style: Style::default(),
            bounds: UtilsRect::ZERO,
            pending_damage: Damage::None,
            is_opaque: true,
            children: header,
        }
    }
}

impl Default for TimeWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl OnChange<DateTimeUpdate> for TimeWidget {
    fn damage(&self, _new: &DateTimeUpdate) -> Damage {
        Damage::None
    }

    fn change(&mut self, new: DateTimeUpdate) {
        self.children.children.0.set(new.time);
        self.children
            .children
            .1
            .set(format!("{} · MECHA COMET", new.date));
    }
}
