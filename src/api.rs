use std::collections::HashMap;
use serde_json::{Map, Value};
use super::table::TableEntry;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum Comp {
    Le(Value),
    Ge(Value),
    Leq(Value),
    Geq(Value),
    Eq(Value),
    Neq(Value),
    In(Vec<Value>),
    Nin(Vec<Value>),
    Between([Value; 2]),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Filter {
    filters: HashMap<String, Value>,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            filters: HashMap::new()
        }
    }

    pub fn insert(&mut self, column: &str, comp: Comp) {
        self.filters.insert(column.to_owned(), serde_json::to_value(comp).unwrap());
    }
    
}

pub enum Selection {
    All,
    Id(i32),
    Filter(Filter),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request error: `{0}`")]
    Request(#[from] reqwest::Error),
    #[error("json error: `{0}`")]
    Json(#[from] serde_json::Error),
    #[error("error: `{0}`")]
    Response(String),
}

#[derive(Debug, Clone)]
pub struct Client {
    url: String,
    client: reqwest::Client,
}

impl Client {
    pub const URL: &str = "http://127.0.0.1:5000";
    
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn response_text(response: reqwest::Response) -> Result<String, Error> {
        let is_success = response.status().is_success();
        
        let text = response.text().await?;

        if is_success { Ok(text) }
        else { Err(Error::Response(text)) }
    }

    pub async fn tables(&self) -> Result<Vec<TableEntry>, Error> {
        let url = format!("{}/api/tables", self.url);
        let response = self.client.get(url)
            .header("Content-Type", "application/json")
            .send().await?;

        let text = Self::response_text(response).await?;

        let tables = serde_json::from_str(&text)?;

        let entries = TableEntry::from_vec(tables);

        Ok(entries)
    }

    pub async fn get(&self, table: &str, selection: Selection) -> Result<Vec<Map<String, Value>>, Error> {
        let url = match &selection {
            Selection::Id(id) => format!("{}/api/item/{}/{}", self.url, table, id),
            _ => format!("{}/api/items/{}", self.url, table),
        };

        let is_single = matches!(selection, Selection::Id(_));

        let body = match &selection {
            Selection::All => Some(serde_json::json!({}).to_string()),
            Selection::Id(_) => None,
            Selection::Filter(filter) => Some(serde_json::to_string(&filter)?),
        };

        let mut builder = self.client
            .get(url)
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            builder = builder.body(body);
        }

        let response = builder.send().await?;
        let text = Self::response_text(response).await?;

        let items = if is_single {
            let value = serde_json::from_str(&text)?;
            vec![value]
        }
        else {
            serde_json::from_str(&text)?
        };

        let items = items.into_iter()
            .map(|item| {
                match item {
                    Value::Object(map) => map,
                    _ => unreachable!(),
                }
            })
            .collect();

        Ok(items)
    }
}
