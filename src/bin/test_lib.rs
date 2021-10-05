use worm_derive::{WormDb, Worm};
use worm::DbContext;
#[derive(WormDb)]
#[db(var(name="TESTDBS"))]
struct TestDb {
    context: DbContext,
}
#[derive(Worm)]
#[dbmodel(table(name="Tests",schema="TestDb",alias="test"))]
struct Test {
    #[dbcolumn(column(name="Id",primary_key))]
    id: i64,
    #[dbcolumn(column(name="Name",insertable))]
    name: String,
}
#[derive(Worm)]
#[dbmodel(table(name="Anothers",schema="TestDb",alias="another"))]
struct Another {
    #[dbcolumn(column(name="Id",primary_key))]
    id: i64,
    #[dbcolumn(column(name="Test_Id",foreign_key="Test",insertable))]
    test_id: i64
}
fn main() {
}
