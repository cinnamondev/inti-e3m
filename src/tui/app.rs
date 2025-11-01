use crate::config::{Config, ServiceProvider};
use crate::server::Server;
use crate::tui::config_option::{ConfigOptType, ConfigOption};
use crate::tui::event::{AppEvent, ErrorKind, Event, EventHandler};
use crate::tui::popup::{DataType, PopupState};
use crate::Command;
use log::{info};
use ratatui::widgets::TableState;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    DefaultTerminal,
};
use crate::tui::bar::ServicesState;
use crate::tui::bar::Status::{NotRunning, Okay, Stopped};

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    pub popup_state: Option<PopupState>,
    pub table_state: TableState,
    pub items: Vec<ConfigOption>,
    pub config: Config,
    /// Counter.
    /// Event handler.
    pub events: EventHandler,
    pub(crate) server: Option<Server>,
    pub services_state: ServicesState,
}


impl App {
    pub fn using_config(config: Config,) -> Self {
        Self {
            server: None,
            running: true,
            popup_state: None,
            items: vec![
                ConfigOption::new(ConfigOptType::PopupInput,"Serial File",
                                  config.machine_config.file.as_str(),
                                  |c| c.machine_config.file.to_string(),
                                  |c,s| { c.machine_config.file = s.to_string(); Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Movement distance",
                                  format!("{} mm", config.machine_config.max_movement).as_str(),
                                  |c| format!("{} mm", c.machine_config.max_movement),
                                  |c,s| { c.machine_config.max_movement = s.parse()?; Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Max throw",
                                  format!("{} mm", config.machine_config.throw).as_str(),
                                  |c| format!("{} mm", c.machine_config.throw),
                                  |c,s| { c.machine_config.throw = s.parse()?; Ok(())}
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Max acceleration",
                                  format!("{} mm/s", config.machine_config.max_acceleration).as_str(),
                                  |c| format!("{} mm/s", c.machine_config.max_acceleration),
                                  |c,s| { c.machine_config.max_acceleration = s.parse()?; Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::PopupInput,"Websocket URI",
                                  config.websocket_config.ws.as_str(),
                                  |c| format!("{} mm/s", c.websocket_config.ws),
                                  |c,s| { c.websocket_config.ws = s.to_string(); Ok(()) }
                ),
                ConfigOption::new(ConfigOptType::Switch,"Service Provider",
                                  config.websocket_config.provider.to_string().as_str(),
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
            config,
            services_state: ServicesState {
                websocket_status: NotRunning,
                usb_status: NotRunning,
                latest_gcode: "".to_string(),
            },
        }
    }
}


impl App {
    pub(crate) fn is_server_running(&self) -> bool {
        if let Some(server) = &self.server {
            !server.handle.is_finished() && !server.tx.is_closed() && !server.token.is_cancelled()
        } else {
            false
        }
    }
    /// Run the application's main loop.
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
                    AppEvent::Server => {
                        if self.is_server_running() {
                            if let Some(server) = &self.server {
                                server.token.cancel();
                                self.server = None;
                                self.services_state.websocket_status = NotRunning;
                                self.services_state.usb_status = NotRunning;
                            }
                        } else  {
                            self.services_state.websocket_status = Okay;
                            self.services_state.usb_status = Okay;
                            self.server = Some(Server::start(self.config.clone(), self.events.sender.clone()));
                        }
                    }
                    AppEvent::Quit => self.quit(),
                    AppEvent::Command(command) if let Some(s) = &self.server => {
                        if !s.tx.is_closed() {
                            if let Err(e) = s.tx.send(command).await {
                                log::error!("Failure sending command to marlin. Error: \n {}", e)
                            }
                        }
                    }
                    AppEvent::GCode(gcode) => self.services_state.latest_gcode = gcode,
                    AppEvent::Command(_) => {},
                    AppEvent::ServerError(e) => {
                        match e {
                            ErrorKind::Websocket(e) => self.services_state.websocket_status = Stopped(e),
                            ErrorKind::GCode(e) => self.services_state.usb_status = Stopped(e),
                        }
                    },
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
            KeyCode::Home => self.events.send(AppEvent::Command(Command::Home)),
            KeyCode::End | KeyCode::Char('h') => self.events.send(AppEvent::Command(Command::Halt)),
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            },
            KeyCode::Char('w') => self.next_row(),
            KeyCode::Char('s') => self.previous_row(),
            KeyCode::Enter if let Some(n) = self.table_state.selected() => { // open popup or submit popup info and see
                if n == self.items.len() {
                    self.events.send(AppEvent::Server);
                } else if !self.is_server_running() {
                    let item: &mut ConfigOption = &mut self.items[n];
                    if item.typ == ConfigOptType::Switch { // the whole switch thing is SUCH a hack... idrc at this point though
                        item.handle(&mut self.config, "");
                    } else {
                        // TODO: popup based off option selecteed.
                        self.popup_state = Some(PopupState {
                            header: item.label.clone(),
                            description: None,
                            entered_text: item.string_repr.clone(),
                            data: DataType::String(0, 30),
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
    pub fn tick(&self) {
        if self.is_server_running() {
            // update 
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

}
