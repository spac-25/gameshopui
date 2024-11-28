#![feature(iter_intersperse)]
#![feature(extract_if)]

mod table;
mod api;

use std::iter;

use iced::{Task, Element, Length, Theme};
use iced::widget::{button, column, container, horizontal_rule, row, scrollable, text, text_input, vertical_rule, Space};
use itertools::Either;
use table::{TableColumn, Table, TableDefinition, TableEntry};
use api::{Client, Selection};

fn main() -> iced::Result {
    iced::application("gameshopui", State::update, State::view)
        .theme(State::theme)
        .run_with(|| {
            let state = StateConnect {
                client: Client::new("http://127.0.0.1:5000".to_owned()),
                state: RequestState::Idle,
                message: None,
            };

            let task = state.task_api_tables().map(Message::Connect);

            let state = State::Connect(state);

            (state, task)
        })
}

#[derive(Debug, Clone)]
enum Message {
    Connect(MessageConnect),
    View(MessageTable)
}

#[derive(Debug)]
enum State {
    Connect(StateConnect),
    View(StateTable),
}

impl State {
    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        if let Message::Connect(MessageConnect::Response(Ok(tables))) = message {
            take_mut::take(self, |state| {
                let state = match state {
                    State::Connect(state) => state,
                    _ => unreachable!(),
                };

                State::View(StateTable {
                    client: state.client,
                    tables,
                    state: RequestState::Idle,
                    message: None,
                    entries: None,
                })
            });

            Task::none()
        }
        else {
            match self {
                State::Connect(state) => {
                    match message {
                        Message::Connect(message) => {
                            state.update(message).map(Message::Connect)
                        },
                        _ => unreachable!(),
                    }
                },
                State::View(state) => {
                    match message {
                        Message::View(message) => {
                            state.update(message).map(Message::View)
                        },
                        _ => unreachable!(),
                    }
                },
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            State::Connect(state) => state.view().map(Message::Connect),
            State::View(state) => state.view().map(Message::View),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RequestState {
    Idle,
    Requesting,
}

#[derive(Debug, Clone)]
enum MessageConnect {
    Edit(String),
    Connect,
    Response(Result<Vec<TableDefinition>, String>),
}

#[derive(Debug)]
struct StateConnect {
    client: Client,
    state: RequestState,
    message: Option<String>,
}

impl StateConnect {
    pub fn update(&mut self, message: MessageConnect) -> iced::Task<MessageConnect> {
        match message {
            MessageConnect::Edit(url) => {
                self.client.url = url;
                self.message = None;

                Task::none()
            },
            MessageConnect::Connect => {
                self.state = RequestState::Requesting;
                self.message = None;

                self.task_api_tables()
            },
            MessageConnect::Response(response) => {
                self.state = RequestState::Idle;

                match response {
                    Ok(_) => unreachable!(),
                    Err(err) => self.message = Some(err),
                }

                Task::none()
            },
        }
    }

    fn task_api_tables(&self) -> Task<MessageConnect> {
        let client = self.client.clone();
        let wrapper = || async move {
            client.tables().await
        };
        iced::Task::perform(
            wrapper(),
            |tables| MessageConnect::Response(tables.map_err(|err| err.to_string())),
        )
    }

    pub fn view(&self) -> Element<MessageConnect> {
        let input = text_input("API URL", &self.client.url)
            .on_input(MessageConnect::Edit)
            .on_submit(MessageConnect::Connect)
            .width(Length::FillPortion(4));

        let button = button(text("Connect").center())
            .on_press_maybe(
                matches!(self.state, RequestState::Idle)
                    .then_some(MessageConnect::Connect)
            )
            .width(Length::FillPortion(1));

        let controls = row![
            input,
            button,
        ]
        .width(512);

        let message = text(
            if let Some(message) = self.message.clone() { message }
            else { String::new() }
        )
        .style(text::danger);

        let column = column![
            Space::with_height(Length::Fill),
            container(controls).center_x(Length::Fill),
            container(message).center_x(Length::Fill).height(Length::Fill),
        ];

        container(column)
            .center(Length::Fill)
            .into()
    }
}

#[derive(Debug, Clone)]
enum MessageTable {
    Entries(MessageEntries),
    GetRequest(String),
    GetResponse(String, Result<Vec<TableEntry>, String>),
}

#[derive(Debug)]
struct StateTable {
    client: Client,
    tables: Vec<TableDefinition>,
    state: RequestState,
    message: Option<String>,
    entries: Option<(String, StateEntries)>,
}

impl StateTable {
    fn get_selected_table(&self) -> Option<&Table> {
        let Some(entries) = self.entries.as_ref() else { return None; };

        self.tables.iter()
            .find_map(|table| table.get(&entries.0))
    }

    pub fn update(&mut self, message: MessageTable) -> iced::Task<MessageTable> {
        match message {
            MessageTable::Entries(message) => {
                let entries = self.entries.as_mut().unwrap();

                let table = self.tables.iter()
                    .find_map(|table| table.get(&entries.0))
                    .unwrap();

                entries.1.update(table, message).map(MessageTable::Entries)
            }
            MessageTable::GetRequest(table) => {
                self.state = RequestState::Requesting;
                self.message = None;

                self.task_api_get(&table, Selection::All)
            },
            MessageTable::GetResponse(table, entries) => {
                self.state = RequestState::Idle;

                match entries {
                    Ok(entries) => {
                        self.entries = Some((
                            table,
                            StateEntries {
                                client: self.client.clone(),
                                entries,
                                state: RequestState::Idle,
                                message: None,
                            },
                        ))
                    },
                    Err(err) => self.message = Some(err),
                }

                Task::none()
            },
        }
    }

    fn task_api_get(&self, table: &str, selection: Selection) -> iced::Task<MessageTable> {
        let client = self.client.clone();
        let table_name = table.to_owned();
        let wrapper = || async move {
            client.get(&table_name, selection).await
        };

        let table_name = table.to_owned();
        iced::Task::perform(
            wrapper(),
            move |get| MessageTable::GetResponse(table_name.clone(), get.map_err(|err| err.to_string())),
        )
    }

    pub fn view(&self) -> Element<MessageTable> {
        let tables: Vec<_> = self.tables.iter()
            .map(|table| {
                match table {
                    TableDefinition::Single(table) => {
                        Either::Left(iter::once(self.view_table(table)))
                    },
                    TableDefinition::Family { base: _, leaves } => {
                        Either::Right(
                            leaves.iter()
                                .map(|table| self.view_table(table))
                        )
                    },
                }
            })
            .flatten()
            .collect();

        let tables = column(tables).width(256);

        let entries = if let Some(entries) = &self.entries {
            let table = self.get_selected_table().unwrap();
            entries.1.view(table).map(MessageTable::Entries)
        }
        else {
            Space::new(Length::Fill, Length::Fill).into()
        };

        row![
            tables,
            vertical_rule(0),
            entries,
        ]
        .into()
    }

    fn view_table(&self, table: &Table) -> Element<MessageTable> {
        let label = text(table.pretty_name())
            .width(Length::Fill)
            .center();

        let idle = matches!(self.state, RequestState::Idle);
        let selected = self.entries.as_ref()
            .map_or(false, |entries| entries.0 == table.table);

        button(label)
            .on_press_maybe((idle && !selected).then_some(MessageTable::GetRequest(table.table.clone())))
            .width(Length::Fill)
            .into()
    }
}

#[derive(Debug, Clone)]
enum MessageEntries {

}

#[derive(Debug)]
struct StateEntries {
    client: Client,
    entries: Vec<TableEntry>,
    state: RequestState,
    message: Option<String>,
}

impl StateEntries {
    pub fn update(&mut self, table: &Table, message: MessageEntries) -> iced::Task<MessageEntries> {
        match message {

        }
    }

    pub fn view(&self, table: &Table) -> Element<MessageEntries> {
        // scrollable(text(format!("{:#?}", self.entries))).width(Length::Fill).into()

        let entries: Vec<_> = table.columns.iter()
            .filter(|column| table.polymorphic.as_ref() != Some(&column.name))
            .map(|column| self.column_view(column))
            .intersperse_with(|| vertical_rule(8).into())
            .collect();

        let entries = row(entries).height(Length::Shrink);

        let direction = scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::new(),
            horizontal: scrollable::Scrollbar::new(),
        };

        scrollable(entries)
            .direction(direction)
            .width(Length::Fill)
            .height(Length::Fill).into()
    }

    fn column_view(&self, column: &TableColumn) -> Element<MessageEntries> {
        let header = text(column.name.clone());

        let values: Vec<_> = self.entries.iter()
            .map(|entry| entry.get(&column.name).unwrap())
            .map(|value| {
                match value {
                    Some(value) => value.to_string(),
                    None => "".to_owned(),
                }
            })
            .map(text)
            .map(Into::into)
            .collect();

        column![
            header,
            horizontal_rule(8),
            iced::widget::column(values),
        ]
        .width(Length::Shrink)
        .into()
    }
}
