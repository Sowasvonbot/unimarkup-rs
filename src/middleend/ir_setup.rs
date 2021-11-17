use crate::middleend::{
    ir_content::ContentIrLine, ir_macros::MacroIrLine, ir_metadata::MetadataIrLine,
    ir_resources::ResourceIrLine, ir_variables::VariableIrLine, IrError,
};
use rusqlite::Connection;

pub fn setup_ir_connection() -> Result<Connection, IrError> {
    Connection::open_in_memory().map_err(|err| IrError {
        tablename: "-".to_string(),
        column: "-".to_string(),
        message: format!("Could not create a database connection. Reason: {:?}", err),
    })
}

pub fn setup_ir(ir_connection: &Connection) -> Result<(), IrError> {
    let sql = format!(
        "{}{}{}{}{}",
        ContentIrLine::table_setup(),
        MacroIrLine::table_setup(),
        VariableIrLine::table_setup(),
        MetadataIrLine::table_setup(),
        ResourceIrLine::table_setup()
    );
    ir_connection.execute_batch(&sql).map_err(|err| IrError {
        tablename: "-".to_string(),
        column: "-".to_string(),
        message: format!(
            "Could not setup tables on given database connection. Reason: {:?}",
            err
        ),
    })
}
