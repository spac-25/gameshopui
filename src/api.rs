use std::collections::HashMap;
use serde_json::Value;
use crate::table::{ColumnValue, Table, TableDefinition, TableEntry};

#[derive(Debug, Clone)]
pub enum Comp<T> {
    Le(T),
    Ge(T),
    Leq(T),
    Geq(T),
    Eq(T),
    Neq(T),
    In(Vec<T>),
    Nin(Vec<T>),
    Between(T, T),
}

impl<T> Comp<T> {
    fn operator(&self) -> &str {
        match self {
            Comp::Le(_) => "<",
            Comp::Ge(_) => ">",
            Comp::Leq(_) => "<=",
            Comp::Geq(_) => ">=",
            Comp::Eq(_) => "==",
            Comp::Neq(_) => "!=",
            Comp::In(_) => "in",
            Comp::Nin(_) => "not_in",
            Comp::Between(_, _) => "range",
        }
    }
}

impl serde::Serialize for Comp<ColumnValue> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let operator = Value::String(self.operator().to_owned());

        let value: Value = match self.clone() {
            Comp::Le(value) => value.into(),
            Comp::Ge(value) => value.into(),
            Comp::Leq(value) => value.into(),
            Comp::Geq(value) => value.into(),
            Comp::Eq(value) => value.into(),
            Comp::Neq(value) => value.into(),
            Comp::In(value) => Value::Array(value.into_iter().map(Into::into).collect()),
            Comp::Nin(value) => Value::Array(value.into_iter().map(Into::into).collect()),
            Comp::Between(min, max) => Value::Array(vec![min.into(), max.into()]),
        };

        let comp = Value::Array(vec![operator, value]);
        comp.serialize(serializer)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Filter(HashMap<String, Comp<ColumnValue>>);

impl Filter {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, column: &str, comp: Comp<ColumnValue>) {
        self.0.insert(column.to_owned(), comp);
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
    pub url: String,
    client: reqwest::Client,
}

impl Client {
    pub fn new(url: String) -> Self {
        Self {
            url: url,
            client: reqwest::Client::new(),
        }
    }

    async fn response_text(response: reqwest::Response) -> Result<String, Error> {
        let is_success = response.status().is_success();

        let text = response.text().await?;

        if is_success { Ok(text) }
        else { Err(Error::Response(text)) }
    }

    pub async fn tables(&self) -> Result<Vec<TableDefinition>, Error> {
        let url = format!("{}/api/tables", self.url);
        let response = self.client.get(url)
            .header("Content-Type", "application/json")
            .send().await?;

        let text = Self::response_text(response).await?;

        let tables = serde_json::from_str(&text)?;

        let entries = TableDefinition::from_vec(tables);

        Ok(entries)
    }

    pub async fn get(&self, table_name: &str, selection: Selection) -> Result<Vec<TableEntry>, Error> {
        // set endpoint based on selection
        let url = match &selection {
            Selection::Id(id) => format!("{}/api/item/{}/{}", self.url, table_name, id),
            _ => format!("{}/api/items/{}", self.url, table_name),
        };

        let is_by_id = matches!(selection, Selection::Id(_));

        let body = match &selection {
            Selection::All => Some(serde_json::json!({}).to_string()), // empty filter to get all entries
            Selection::Id(_) => None, // by id endpoint has no body
            Selection::Filter(filter) => Some(serde_json::to_string(filter)?), // use filter
        };

        let mut builder = self.client
            .get(url)
            .header("Content-Type", "application/json");

        // include body if there is one
        if let Some(body) = body {
            builder = builder.body(body);
        }

        let response = builder.send().await?;
        let text = Self::response_text(response).await?;

        // handle single/multiple entries
        let items = if is_by_id {
            let value = serde_json::from_str(&text)?;
            vec![value]
        }
        else {
            serde_json::from_str(&text)?
        };


        let items = items.into_iter()
            .map(|item| {
                let map = match item {
                    Value::Object(map) => map,
                    _ => unreachable!(),
                };

                map.into_iter()
                    .map(|(k, v)| {
                        (k, ColumnValue::try_from_value(v).unwrap())
                    })
                    .collect()
            })
            .collect();

        Ok(items)
    }
}
