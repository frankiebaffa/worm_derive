use {
    darling::{
        ast,
        FromMeta,
        FromField,
        FromDeriveInput,
        ToTokens,
    },
    proc_macro::TokenStream,
    quote::{
        quote,
        TokenStreamExt,
    },
    syn::{
        DeriveInput,
        parse_macro_input,

    },
};
#[derive(Default, Debug, FromMeta)]
struct DbModelColumn {
    name: String,
    #[darling(default)]
    active_flag: bool,
}
#[derive(FromField)]
#[darling(attributes(dbcolumn))]
struct DbModelColumnOpts {
    column: DbModelColumn,
    ident: Option<syn::Ident>,
}
#[derive(Default, FromMeta)]
struct DbModelTable {
    db: String,
    name: String,
    alias: String,
}
#[derive(FromDeriveInput)]
#[darling(attributes(dbmodel))]
struct DbModelOpts {
    table: DbModelTable,
    ident: syn::Ident,
    data: ast::Data<(), DbModelColumnOpts>,
}
#[proc_macro_derive(Worm, attributes(dbmodel, dbcolumn))]
pub fn derive_dbmodel(input: TokenStream) -> TokenStream {
    let d_input = parse_macro_input!(input as DeriveInput);
    let opts = DbModelOpts::from_derive_input(&d_input).unwrap();
    let db = opts.table.db.as_str();
    let table = opts.table.name.as_str();
    let alias = opts.table.alias.as_str();
    let name = opts.ident;
    let mut columns = Vec::new();
    let mut idents = Vec::new();
    let mut vars = Vec::new();
    let data = opts.data.take_struct().unwrap();
    let mut has_active_flag = false;
    let mut active = None;
    for field in data {
        let ident = field.ident.unwrap();
        let column = field.column;
        let var = ident.to_token_stream();
        columns.push(column.name.clone());
        idents.push(ident.clone());
        vars.push(var);
        let active_flag = column.active_flag;
        if active_flag && !has_active_flag {
            has_active_flag = true;
            active = Some((column.name, ident));
        } else if active_flag && has_active_flag {
            panic!("A table cannot contain more than one active flag");
        }
    }
    let mut traits = quote!{};
    let dbmodel_trait = quote! {
        impl worm::traits::dbmodel::DbModel for #name {
            const DB: &'static str = #db;
            const TABLE: &'static str = #table;
            const ALIAS: &'static str = #alias;
            fn from_row(row: &rusqlite::Row) -> Result<#name, rusqlite::Error>{
                use worm::traits::helpers::ColumnValue;
                #(let #idents = row.value(&#columns)?;)*
                return Ok(#name { #(#idents: #idents, )*});
            }
        }
    };
    dbmodel_trait.to_tokens(&mut traits);
    if active.is_some() {
        let active_res = active.unwrap();
        let value = active_res.0;
        let key = active_res.1;
        let activeflag_trait = quote! {
            impl worm::traits::activeflag::ActiveFlag for #name {
                const ACTIVE: &'static str = #value;
                fn get_active(&self) -> bool {
                    return self.#key;
                }
            }
        };
        activeflag_trait.to_tokens(&mut traits);
    }
    traits.into()
}

