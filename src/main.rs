#![feature(iter_intersperse)]
#![feature(extract_if)]

mod table;

use iced::widget::{scrollable, column, button, text};
use table::{Table, TableEntry};

fn main() -> iced::Result {
    iced::run("gameshopui", GameShop::update, GameShop::view)
}

#[derive(Debug, Default)]
struct GameShop {
    response: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Message {
    Request,
    Tables(Result<Vec<Table>, String>),
    Items(Result<Vec<serde_json::Value>, String>)
}

async fn get_tables() -> Result<Vec<Table>, String> {
    let client = reqwest::Client::new();
    let response = client.get("http://127.0.0.1:5000/api/tables")
        .header("Content-Type", "application/json")
        .send().await
        .map_err(|e| e.to_string())?;

    let text = response.text().await
        .map_err(|e| e.to_string())?;

    let tables = serde_json::from_str(&text)
        .map_err(|e| e.to_string())?;

    Ok(tables)
}

async fn get_items() -> Result<Vec<serde_json::Value>, String> {
    let client = reqwest::Client::new();
    let response = client.get("http://127.0.0.1:5000/api/items/item")
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({
                // "id": ["range", [1, 3]],
                // // "id": ["range", [1, 2]],
                // "price": [">", 40],
                // // "name": ["in", ["Settlers of Cataan", "Ticket to ride"]],
            })
            .to_string()
        )
        .send().await
        .map_err(|e| e.to_string())?;

    let text = response.text().await
        .map_err(|e| e.to_string())?;

    let items = serde_json::from_str(&text)
        .map_err(|e| e.to_string())?;

    Ok(items)
}

impl GameShop {
    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Request => {
                iced::Task::perform(get_tables(), Message::Tables)
            },
            Message::Tables(response) => {
                let text = match response {
                    Ok(tables) => {
                        let entries = TableEntry::from_vec(tables);
                        format!("{:#?}", entries)
                    },
                    Err(err) => err,
                };

                self.response = Some(text);
                iced::Task::none()
            },
            Message::Items(response) => {
                let text = match response {
                    Ok(items) => {
                        format!("{:#?}", items)
                    },
                    Err(err) => err,
                };

                self.response = Some(text);
                iced::Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::widget::Scrollable<Message> {
        scrollable(column![
            button("Request").on_press(Message::Request),
            text(self.response.as_ref().map_or("", String::as_str)).size(16),
        ].width(iced::Length::Fill))
    }
}
