use crate::il2cpp::types::Il2CppImage;

pub mod Connection;
mod PreparedQuery;
pub mod Query;

pub fn init(image: *const Il2CppImage) {
    Query::init(image);
    PreparedQuery::init(image);
    Connection::init(image);
}
