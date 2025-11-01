use core::error;
use std::error::Error;
use std::fmt::Debug;
use ratatui::widgets::Row;
use crate::config::Config;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ConfigOptType {
    Switch,
    PopupInput
}
pub struct ConfigOption {
    pub typ: ConfigOptType,
    pub label: String,
    pub string_repr: String,
    pub update_str: Box<dyn Fn(&Config) -> String>,
    pub function: Box<dyn FnMut(&mut Config, &str) -> Result<(), Box<dyn error::Error>>>,
}

impl Debug for ConfigOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { // skip function because its undebuggable
        f.debug_struct("ConfigOption")
            .field("label", &self.label)
            .finish()
    }
}
#[derive(Debug)]
pub enum ConfigParseError {

}
impl ConfigOption {
    pub fn new<F,T>(typ: ConfigOptType, label: &str, initial_repr: &str, string_getter: T, function: F) -> ConfigOption
    where F: FnMut(&mut Config, &str) -> Result<(), Box<dyn Error>> + 'static, T: Fn(&Config) -> String + 'static  {
        ConfigOption {
            typ,
            label: label.to_string(),
            string_repr: initial_repr.to_string(),
            update_str: Box::new(string_getter),
            function: Box::new(function),
        }
    }

    pub fn handle(&mut self, config: &mut Config, input_string: &str) -> Result<(), Box<dyn error::Error>> {
        let r = (self.function)(config, input_string);
        self.string_repr = (self.update_str)(config);
        r
    }

}

impl<'a> From<&ConfigOption> for Row<'a> {
    fn from(value: &ConfigOption) -> Self {
        Row::new(vec![value.label.clone(), value.string_repr.clone()])
    }
}