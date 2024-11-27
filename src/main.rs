#![feature(iter_intersperse)]
#![feature(extract_if)]

mod table;
mod api;

use std::collections::HashMap;

use serde_json::{Map, Value};

use iced::{widget::{button, column, scrollable, text}, Element};
use table::TableEntry;
use api::Client;

fn main() -> iced::Result {
    iced::application("gameshopui", State::update, State::view)
        .run_with(|| {
            let state = State {
                client: Client::new(Client::URL),
                cache: HashMap::new(),
                entries: EntriesState::Fetching,
            };

            let task = state.fetch_tables_task();
            
            (state, task)
        })
}

#[derive(Debug, Clone)]
enum EntriesState {
    Inactive,
    Fetching,
    Entries(Vec<TableEntry>)
}

#[derive(Debug)]
struct State {
    client: Client,
    cache: HashMap<String, Vec<Map<String, Value>>>,
    entries: EntriesState,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Message {
    FetchTables,
    Tables(Result<Vec<TableEntry>, String>),
    Items(Result<Vec<Map<String, Value>>, String>)
}

impl State {
    fn fetch_tables_task(&self) -> iced::Task<Message> {
        let client = self.client.clone();
        let wrapper = || async move {
            client.tables().await
        };
        iced::Task::perform(
            wrapper(),
            |items| Message::Tables(items.map_err(|err| err.to_string())),
        )
        
    }
    
    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::FetchTables => {
                self.entries = EntriesState::Fetching;
                self.fetch_tables_task()
            },
            Message::Tables(response) => {
                match response {
                    Ok(entries) => {
                        self.entries = EntriesState::Entries(entries);
                    },
                    Err(err) => {
                        self.entries = EntriesState::Inactive;
                    },
                };

                iced::Task::none()
            },
            Message::Items(response) => {
                iced::Task::none()
            },
        }
    }

    pub fn view(&self) -> iced::widget::Scrollable<Message> {
        scrollable(self.view_entries())
    }

    fn view_entries(&self) -> Element<Message> {
        let fetch_tables_message = match self.entries {
            EntriesState::Inactive => Some(Message::FetchTables),
            _ => None,
        };

        match &self.entries {
            EntriesState::Inactive | EntriesState::Fetching => {
                button("Fetch tables").on_press_maybe(fetch_tables_message).into()
            },
            EntriesState::Entries(entries) => {
                let names: Vec<_> = entries.iter()
                    .map(|entry| {
                        let name = entry.get_base().pretty_name();
                        text(name).into()
                    })
                    .collect();
                column(names).into()
            },
        }
        
    }
}
