use ratatui::buffer::Buffer;
use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::prelude::{Stylize, Widget};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, StatefulWidget, Wrap};
use crate::tui::app::App;

pub struct Popup;

#[derive(Debug)]
pub enum DataType {
    INTEGER(i32,i32),
    UNSIGNED_INTEGER(u32,u32),
    STRING(usize, usize),
}

impl DataType {
    pub fn get_max_chars(&self) -> usize {
         match self {
            DataType::INTEGER(lower, upper) => { upper.to_string().len() }
            DataType::UNSIGNED_INTEGER(lower, upper) => { upper.to_string().len() }
            DataType::STRING(min, max) => *max
        }
    }
}

#[derive(Debug)]
pub struct PopupState {
    pub(crate) header: String,
    pub(crate) description: Option<String>,
    pub(crate) entered_text: String,
    pub(crate) data: DataType,
}

impl StatefulWidget for Popup {
    type State = PopupState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut entry_width = state.data.get_max_chars();
        let entry_width= (entry_width + 6).min(20) as u16;

        let (paragraph, additional_height) = if let Some(description) = &state.description {
            let paragraph = Paragraph::new(description.as_str())
                .centered()
                .wrap(Wrap { trim: true });
            let lines = paragraph.line_count(entry_width);
            (Some(paragraph),lines)
        } else { (None,0) };

        let area = popup_area(area, entry_width, (2+ 1 + 1 + additional_height) as u16);
        Clear.render(area, buf);

        let block = Block::bordered().title(state.header.as_str());
        block.render(area, buf);

        let [paragraph_area, entry_area, controls_area] = Layout::vertical([Constraint::Min(0), Constraint::Length(1), Constraint::Length(1)])
            .vertical_margin(1).horizontal_margin(2).areas(area);

        if let Some(paragraph) = paragraph {
            paragraph.render(paragraph_area, buf);
        }
        let line = Line::from(state.entered_text.as_str()).slow_blink();
        line.render(entry_area, buf);

        let controls = Line::from(vec![
            Span::from(" Cancel "), Span::from(" Esc "),
            Span::from(" Submit "), Span::from(" Enter "),
        ]).centered();
        controls.render(controls_area, buf);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(width)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

impl<'a> App<'a> {
    pub fn entry_popup(frame: &mut Frame, header: &str, label: &str, width: usize) {

    }
}

