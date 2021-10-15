use worm_derive::{WormDb, Worm};
use worm::DbContext;
use worm::builder::Query;
use worm::builder::WormError;
use worm::traits::primarykey::PrimaryKey;
use worm::builder::JoinFK;
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
    let test = Query::<Test>::select()
        .join_pk::<Another>()
        .join_eq::<Another>(Another::ID, &1)
        .where_eq(Test::NAME, &"Hello");
    println!("{}", test.query_to_string());
    let another = Query::<Another>::select()
        .join_fk();
    println!("{}", another.query_to_string());
}
