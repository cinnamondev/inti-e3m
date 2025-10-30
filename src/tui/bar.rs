use color_eyre::owo_colors::OwoColorize;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Stylize};
use ratatui::style::Color::Gray;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget};

pub struct Bar;

pub struct ServicesState {
    pub websocket_status: bool,
    pub usb_status: bool,
    pub latest_gcode: String,
}
impl Bar {
    fn render_left(&self, area: Rect, buf: &mut Buffer, state: &mut ServicesState) {
        area.width;

        Line::from(vec![
            Span::from(" WebSocket "),
            if state.websocket_status {
                Span::from("Yay!").style((Color::Green, Color::LightGreen))
            } else { Span::from("No :(").style((Color::Red, Color::LightRed)) },

            Span::from(" USB "),
            if state.usb_status {
                Span::from("Yay!").style((Color::Green, Color::LightGreen))
            } else { Span::from("No :(").style((Color::Red, Color::LightRed)) }
        ]).render(area, buf);
    }
    fn render_right(&self, area: Rect, buf: &mut Buffer, state: &mut ServicesState) {
        let span = if state.latest_gcode.is_empty() {
            Span::from("No GCode Sent")
        } else {
            Span::from(format!("Last Command: {}", state.latest_gcode))
        };
        Line::from(span).right_aligned().render(area, buf);
    }
    fn render_centre(&self, area: Rect, buf: &mut Buffer, state: &mut ServicesState) { // controls !
        let keys = [
            ("w/↑", "Up"),
            ("s/↓", "Down"),
            ("Enter", "Edit"),
            ("H/End", "Halt"),
            ("X/Esc", "Quit"),
        ];
        let spans: Vec<_> = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::raw(format!(" {key} "))
                    .fg(Color::Yellow)
                    .bg(Color::Blue);
                let desc = Span::raw(format!(" {desc} "))
                    .fg(Color::Yellow)
                    .bg(Color::Blue);
                [key, desc]
            })
            .collect();
        Line::from(spans)
            .centered()
            .bg(Color::Gray)
            .render(area, buf);
    }
}

impl StatefulWidget for Bar {
    type State = ServicesState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let horizontal = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(60),
            Constraint::Min(0),
        ]);
        let [left_bar, centre, right_bar] = horizontal.areas(area);

        Block::new().style((Color::Gray,Color::Gray)).render(area, buf);
        self.render_left(left_bar, buf, state);
        self.render_centre(centre, buf, state);
        self.render_right(right_bar, buf, state);
    }
}