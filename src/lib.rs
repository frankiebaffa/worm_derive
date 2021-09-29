use {
    darling::{
        ast,
        FromMeta,
        FromField,
        FromDeriveInput,
        ToTokens,
    },
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        DeriveInput,
        parse_macro_input,
    },
};
#[derive(Default, FromMeta)]
struct DbModelTable {
    db: String,
    name: String,
    alias: String,
}
#[derive(Default, Debug, FromMeta)]
struct DbModelColumn {
    name: String,
}
#[derive(FromField)]
#[darling(attributes(dbcolumn))]
struct DbModelColumnOpts {
    column: DbModelColumn,
    ident: Option<syn::Ident>,
}
#[derive(FromDeriveInput)]
#[darling(attributes(dbmodel))]
struct DbModelOpts {
    table: DbModelTable,
    ident: syn::Ident,
    data: ast::Data<(), DbModelColumnOpts>,
}
#[proc_macro_derive(Worm, attributes(dbmodel, dbcolumn))]
pub fn derive_worm(input: TokenStream) -> TokenStream {
    let d_input = parse_macro_input!(input as DeriveInput);
    let opts = DbModelOpts::from_derive_input(&d_input).unwrap();
    let db = opts.table.db.as_str();
    let table = opts.table.name.as_str();
    let alias = opts.table.alias.as_str();
    let name = opts.ident;
    let mut columns = Vec::new();
    let mut idents = Vec::new();
    let data = opts.data.take_struct().unwrap();
    for field in data {
        let ident = field.ident.unwrap();
        let column = field.column;
        columns.push(column.name);
        idents.push(ident);
    }
    (quote! {
        impl DbModel2 for #name {
            const DB: &'static str = #db;
            const TABLE: &'static str = #table;
            const ALIAS: &'static str = #alias;
            fn from_row2() {
                #(let #idents = #columns;)*
            }
        }
    }).into()
}

