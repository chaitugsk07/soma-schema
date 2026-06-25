use serde::Deserialize;
use soma_ui::{SchemaColumn, SchemaRelation, SchemaTable};

// These structs mirror the migrations.json schema; not all fields are rendered.
#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct MigrationEntry {
    pub order_index: usize,
    pub version: u32,
    pub file: String,
    pub name: String,
    pub checksum: String,
    pub created: String,
    pub author: String,
    pub why: String,
    pub up_sql: String,
    pub down_sql: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct VersionBlock {
    pub version: u32,
    pub migrations: Vec<MigrationEntry>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct OrderRef {
    pub order_index: usize,
    pub version: u32,
    pub file: String,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct FkJson {
    pub table: String,
    pub column: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct ColumnJson {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    #[serde(default)]
    pub pk: bool,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub unique: bool,
    pub fk: Option<FkJson>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct TableJson {
    pub name: String,
    pub schema: Option<String>,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub columns: Vec<ColumnJson>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct RelationJson {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Schema {
    #[serde(default)]
    pub tables: Vec<TableJson>,
    #[serde(default)]
    pub relations: Vec<RelationJson>,
}

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct MigrationsData {
    pub generated_for: String,
    pub versions: Vec<VersionBlock>,
    pub apply_order: Vec<OrderRef>,
    pub rollback_order: Vec<OrderRef>,
    pub setup_files: Vec<String>,
    #[serde(default)]
    pub schema: Schema,
}

static MIGRATIONS_JSON: &str = include_str!("../data/migrations.json");

pub fn load_migrations() -> MigrationsData {
    serde_json::from_str(MIGRATIONS_JSON).expect("invalid migrations.json")
}

pub fn load_schema() -> (Vec<SchemaTable>, Vec<SchemaRelation>) {
    let data = load_migrations();
    let tables = data
        .schema
        .tables
        .into_iter()
        .map(|t| SchemaTable {
            name: t.name,
            schema: t.schema,
            x: t.x,
            y: t.y,
            columns: t
                .columns
                .into_iter()
                .map(|c| SchemaColumn {
                    name: c.name,
                    col_type: c.col_type,
                    pk: c.pk,
                    fk: c.fk.is_some(),
                    nullable: c.nullable,
                    unique: c.unique,
                })
                .collect(),
        })
        .collect();
    let relations = data
        .schema
        .relations
        .into_iter()
        .map(|r| SchemaRelation {
            from_table: r.from_table,
            from_column: r.from_column,
            to_table: r.to_table,
            to_column: r.to_column,
        })
        .collect();
    (tables, relations)
}
