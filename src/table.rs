use std::collections::HashMap;

use serde_json::{Number, Value};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum ColumnType {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "str")]
    String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ColumnValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl PartialEq for ColumnValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for ColumnValue {}

impl From<ColumnValue> for Value {
    fn from(value: ColumnValue) -> Self {
        match value {
            ColumnValue::Bool(value) => Value::Bool(value),
            ColumnValue::Int(value) => Value::Number(Number::from(value)),
            ColumnValue::Float(value) => Value::Number(Number::from_f64(value).unwrap()),
            ColumnValue::String(value) => Value::String(value),
        }
    }
}

impl std::fmt::Display for ColumnValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            ColumnValue::Bool(value) => value.to_string(),
            ColumnValue::Int(value) => value.to_string(),
            ColumnValue::Float(value) => value.to_string(),
            ColumnValue::String(value) => value.to_string(),
        };

        f.write_str(&string)
    }
}

impl ColumnValue {
    pub fn ty(&self) -> ColumnType {
        match self {
            ColumnValue::Bool(_) => ColumnType::Bool,
            ColumnValue::Int(_) => ColumnType::Int,
            ColumnValue::Float(_) => ColumnType::Float,
            ColumnValue::String(_) => ColumnType::String,
        }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum ColumnParseError {
    #[error("invalid variant")]
    ValueError,
    #[error("empty string")]
    Empty,
    #[error("invalid string")]
    ParseError,
}

impl ColumnValue {
    pub fn try_from_value(value: Value) -> Result<Option<ColumnValue>, ()> {
        match value {
            Value::Null => Ok(None),
            Value::Bool(value) => Ok(Some(ColumnValue::Bool(value))),
            Value::Number(number) => {
                let value = if let Some(value) = number.as_f64() {
                    ColumnValue::Float(value)
                }
                else if let Some(value) = number.as_i64() {
                    ColumnValue::Int(value)
                }
                else {
                    ColumnValue::Int(number.as_u64().unwrap() as i64)
                };

                Ok(Some(value))
            },
            Value::String(value) => Ok(Some(ColumnValue::String(value))),
            Value::Array(_) => Err(()),
            Value::Object(_) => Err(()),
        }
    }

    pub fn try_from_str(column: TableColumn, value: &str) -> Result<Option<ColumnValue>, ColumnParseError> {
        if value == "" {
            return if column.optional {
                Ok(None)
            }
            else if column.ty == ColumnType::String {
                Ok(Some(ColumnValue::String(String::new())))
            }
            else {
                Err(ColumnParseError::Empty)
            };
        }

        let value = match column.ty {
            ColumnType::Bool => value.parse().map(ColumnValue::Bool).map_err(|_| ColumnParseError::ParseError),
            ColumnType::Int => value.parse().map(ColumnValue::Int).map_err(|_| ColumnParseError::ParseError),
            ColumnType::Float => value.parse().map(ColumnValue::Float).map_err(|_| ColumnParseError::ParseError),
            ColumnType::String => Ok(ColumnValue::String(value.to_owned())),
        };

        value.map(Some)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TableColumnForeignKey {
    pub table: String,
    pub column: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TableColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ColumnType,
    pub optional: bool,
    pub primary_key: bool,
    pub foreign_keys: Vec<TableColumnForeignKey>,
    pub mapper: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Table {
    pub name: String,
    pub table: String,
    pub polymorphic: Option<String>,
    pub columns: Vec<TableColumn>,
}

impl Table {
    pub fn pretty_name(&self) -> String {
        let mut name = self.table.clone().replace('_', " ");
        let mut chars = name.chars();
        if let Some(first) = chars.next() {
            name = first.to_uppercase().chain(chars).collect();
        }

        name
    }
}

#[derive(Debug, Clone)]
pub enum TableDefinition {
    Single(Table),
    Family { base: Table, leaves: Vec<Table> },
}

impl TableDefinition {
    pub fn from_vec(tables: Vec<Table>) -> Vec<Self> {
        let (trees, _) = TableNode::into_trees(tables);

        trees.into_iter()
            .map(|mut tree| {
                if let Some(leaves) = tree.pop_outer_leaves() {
                    TableDefinition::Family { base: tree.node, leaves }
                }
                else {
                    TableDefinition::Single(tree.node)
                }
            })
            .collect()
    }

    pub fn get_base(&self) -> &Table {
        match self {
            TableDefinition::Single(table) => table,
            TableDefinition::Family { base, leaves: _ } => base,
        }
    }

    pub fn get_leaves(&self) -> Option<&Vec<Table>> {
        match self {
            TableDefinition::Single(_) => None,
            TableDefinition::Family { base: _, leaves } => Some(leaves),
        }
    }

    pub fn get(&self, table_name: &str) -> Option<&Table> {
        let base = self.get_base();
        if base.table == table_name {
            Some(base)
        }
        else {
            self.get_leaves().and_then(|leaves| {
                leaves.iter().find(|table| table.table == table_name)
            })
        }
    }
}

pub type TableEntry = HashMap<String, Option<ColumnValue>>;

#[derive(Debug, Clone)]
struct TableNode {
    node: Table,
    leaves: Vec<TableNode>,
}

impl TableNode {
    fn construct_leaves(node: &Table, tables: &mut Vec<Table>) -> Vec<TableNode> {
        // find tables whose primary key is a foreign key to the node
        let leaves: Vec<_> = tables
            .extract_if(|table| {
                let id = table.columns.iter()
                    .find(|column| column.primary_key);

                if let Some(column) = id {
                    column.foreign_keys.iter()
                        .map(|key| &key.table)
                        .find(|key_table| key_table == &&node.table)
                        .is_some()
                }
                else {
                    false
                }
            })
            .collect();

        // recursively construct higher leaves
        leaves.into_iter()
            .map(|table| {
                TableNode {
                    leaves: Self::construct_leaves(&table, tables),
                    node: table
                }
            })
            .collect()
    }

    fn into_trees(mut tables: Vec<Table>) -> (Vec<Self>, Vec<Table>) {
        // find base tables (primary key is not a foreign key)
        let bases: Vec<_> = tables
            .extract_if(|table| {
                table.columns.iter()
                    .find(|column| column.primary_key)
                    .map_or(false, |column| column.foreign_keys.is_empty())
            })
            .collect();

        // contruct trees from bases
        let trees = bases.into_iter()
            .map(|table| {
                TableNode {
                    leaves: Self::construct_leaves(&table, &mut tables),
                    node: table
                }
            })
            .collect();

        (trees, tables)
    }

    fn pop_outer_leaves(&mut self) -> Option<Vec<Table>> {
        if self.leaves.is_empty() {
            return None;
        }

        // if a leaf popped some leaves, use those, otherwise pop the leaf
        let mut child_leaves = Vec::new();
        let empty_leaves: Vec<_> = self.leaves.extract_if(|leaf| {
                match leaf.pop_outer_leaves() {
                    Some(leaves) => {
                        child_leaves.extend(leaves);
                        false
                    },
                    None => true,
                }
            })
            .map(|node| node.node)
            .collect();

        // join both types of popped leaves
        child_leaves.extend(empty_leaves);

        Some(child_leaves)
    }
}
