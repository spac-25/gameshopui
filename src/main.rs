fn main() -> iced::Result {
    iced::run("A cool counter", GameShop::update, GameShop::view)
}

#[derive(Default)]
struct GameShop {
    response: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Request,
    Response(Result<serde_json::Value, String>),
}

async fn get() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let response = client.get("http://127.0.0.1:5000/api/tables/item")
        .header("Content-Type", "application/json")
        // .body(
        //     serde_json::json!({
        //     })
        //     .to_string()
        // )
        .send().await
        .map_err(|e| e.to_string())?;

    let text = response.text().await
        .map_err(|e| e.to_string())?;

    let json = serde_json::from_str(&text)
        .map_err(|e| e.to_string())?;

    Ok(json)
}

use iced::widget::{scrollable, column, button, text};

impl GameShop {
    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Request => {
                iced::Task::perform(get(), Message::Response)
            },
            Message::Response(response) => {
                let text = match response {
                    Ok(json) => format!{"{json:#?}"},
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
            text(self.response.as_ref().map_or("", String::as_str)).size(12),
        ].width(iced::Length::Fill))
    }
}