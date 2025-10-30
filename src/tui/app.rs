use std::cmp::PartialEq;
use crate::tui::event::{AppEvent, Event, EventHandler};
use crate::tui::popup::{DataType, PopupState};
use ratatui::widgets::TableState;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    DefaultTerminal,
};
use std::fmt::Debug;
use log::{info, log};
use crate::tui::config::{Config, ServiceProvider};
use crate::tui::config_option::{ConfigOptType, ConfigOption};

/// Application.
#[derive(Debug)]
pub struct App<'a> {
    /// Is the application running?
    pub running: bool,
    pub server_running: bool,
    pub popup_state: Option<PopupState>,
    pub table_state: TableState,
    pub items: Vec<ConfigOption>,
    pub config: Config<'a>,
    /// Counter.
    /// Event handler.
    pub events: EventHandler,
}

impl<'a> App<'a> {
    pub fn using_config(config: Config<'a>) -> Self {
        todo!()
    }
}


impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self {
            server_running: false,
            running: true,
            popup_state: None,
            items: vec![
                ConfigOption::new(ConfigOptType::PopupInput,"Serial File",
                                  "/dev/ttyUSB0",
                                  |c| c.machine_config.file.to_string(),
                                  |c,s| { Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Movement distance",
                                  "240 mm",
                                  |c| format!("{} mm", c.machine_config.max_movement),
                                  |c,s| { Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Max throw",
                                  "100 mm",
                                  |c| format!("{} mm", c.machine_config.throw),
                                  |c,s| { Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Max acceleration",
                                  "1000 mm/s",
                                  |c| format!("{} mm/s", c.machine_config.max_acceleration),
                                  |c,s| { Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Websocket URI",
                                  "ws://localhost:8080",
                                  |c| format!("{} mm/s", c.websocket_config.ws),
                                  |c,s| { Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::Switch,"Service Provider",
                                  "INTI",
                                  |c| c.websocket_config.provider.to_string(),
                                  |c,s| {
                                      c.websocket_config.provider = if c.websocket_config.provider == ServiceProvider::INTI {
                                              ServiceProvider::EXTOY
                                      } else {
                                          ServiceProvider::INTI
                                      };
                                      Ok(())
                                  }
                )
            ],
            table_state: TableState::default().with_selected(0).with_selected_column(1),
            events: EventHandler::new(),
            config: Default::default(),
        }
    }
}

impl<'a> App<'a> {
    /// Run the application's main loop.
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| self.draw(frame));
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press => {
                        self.handle_key_events(key_event)?
                    }
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::Halt | AppEvent::Home => todo!(),
                    AppEvent::GCode(_) => todo!(),
                },
            }
        }
        Ok(())
    }

    pub fn next_row(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.items.len() {
                    0
                } else {
                    i + 1
                }
            }, None => 0
        };
        self.table_state.select(Some(i));
    }
    pub fn previous_row(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() // -1 // removed to allow stop button
                } else {
                    i - 1
                }
            }, None => 0
        };
        self.table_state.select(Some(i));
    }
    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            // popup logic / alternative controls :)
            KeyCode::Char(c) if let Some(popup) = self.popup_state.as_mut() => {
                if popup.entered_text.len() < popup.data.get_max_chars() {
                    popup.entered_text.push(c);
                }
            }
            KeyCode::Esc if self.popup_state.is_some() => {
                self.popup_state = None;
            }
            KeyCode::Enter if let Some(popup) = &self.popup_state => {
                match self.table_state.selected() {
                    Some(i) => self.items[i].handle(&mut self.config, popup.entered_text.as_str()).expect(""),
                    None => {},
                };
                self.popup_state = None;
            },
            KeyCode::Backspace if let Some(popup) = self.popup_state.as_mut() => {
                popup.entered_text.pop();
            }
            // normal controls
            KeyCode::End | KeyCode::Char('h') => self.events.send(AppEvent::Halt),
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            },
            KeyCode::Char('w') => self.next_row(),
            KeyCode::Char('s') => self.previous_row(),
            KeyCode::Enter if let Some(n) = self.table_state.selected() => { // open popup or submit popup info and see
                if n == self.items.len() {
                    self.server_running = !self.server_running;
                } else if !self.server_running {
                    let item: &mut ConfigOption = &mut self.items[n];
                    if item.typ == ConfigOptType::Switch { // the whole switch thing is SUCH a hack... idrc at this point though
                        item.handle(&mut self.config, "");
                    } else {
                        // TODO: popup based off option selecteed.
                        self.popup_state = Some(PopupState {
                            header: item.label.clone(),
                            description: None,
                            entered_text: item.string_repr.clone(),
                            data: DataType::STRING(10, 15),
                        });
                    }
                } else {
                    info!("Config options cannot be changed while server is alive.");
                }
            }
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

}
