use crate::tui::app::App;
use crate::tui::bar::{Bar, ServicesState};
use crate::tui::popup::Popup;
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::widgets::{Row, Table};
use ratatui::{layout::{Alignment, Rect}, style::Color, widgets::{Block, BorderType}, Frame};
use tui_logger::TuiLoggerWidget;

impl App {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
        ]);

        let [main, bar] = layout.areas(frame.area());
        frame.render_widget(Block::bordered()
            .title("Inti-E3M")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded), main);

        let [config, log] = Layout::horizontal([Constraint::Length(60), Constraint::Min(10)]).margin(2).flex(Flex::Center).areas(main);
        self.render_table(frame,config);
        frame.render_stateful_widget(Bar, bar, &mut self.services_state);

        frame.render_widget(
            TuiLoggerWidget::default()
                .block(Block::bordered().title("Log")),
            log
        );

        if let Some(popup) = self.popup_state.as_mut() {
            frame.render_stateful_widget(Popup, frame.area(), popup);
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .title("Options")
            .border_type(BorderType::Rounded);
        let mut v = self.items.iter()
            .map(|c| Row::from(c))
            .collect::<Vec<Row>>();
        v.push(Row::new(["",
            if !self.is_server_running() { " Start Server " } else { " Stop Server " }
        ]));
        let table = Table::new(
            v,
            [Constraint::Min(10), Constraint::Min(15)]
        )
            .block(block)
            .row_highlight_style((Color::Red, Color::Magenta))
            .cell_highlight_style((Color::Green, Color::Blue));

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }
}
