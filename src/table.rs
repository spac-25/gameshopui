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
    pub ty: String,
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
pub enum TableEntry {
    Single(Table),
    Family { base: Table, leaves: Vec<Table> },
}

impl TableEntry {
    pub fn from_vec(tables: Vec<Table>) -> Vec<Self> {
        let (trees, _) = TableNode::into_trees(tables);

        trees.into_iter()
            .map(|mut tree| {
                if let Some(leaves) = tree.pop_outer_leaves() {;
                    TableEntry::Family { base: tree.node, leaves }
                }
                else {
                    TableEntry::Single(tree.node)
                }
            })
            .collect()
    }
}

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
