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
    bool_flag: Option<String>,
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
    let mut bool_flag_ident = None;
    let bool_flag = opts.table.bool_flag;
    let data = opts.data.take_struct().unwrap();
    for field in data {
        let ident = field.ident.unwrap();
        let column = field.column;
        let var = ident.to_token_stream();
        columns.push(column.name.clone());
        idents.push(ident.clone());
        vars.push(var);
        if !bool_flag.is_some() {
            let bool_flag_res = bool_flag.clone().unwrap();
            if bool_flag_res.eq(&column.name.clone()) {
                bool_flag_ident = Some(ident);
            }
        }
    }
    let mut traits = quote! {};
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
    traits.append_all(dbmodel_trait);
    if !bool_flag_ident.is_some() && !columns.contains(&name.to_string()) {
        let bool_flag_ident_res = bool_flag_ident.unwrap();
        let boolflag_trait = quote! {
            //impl worm::traits::activeflag::ActiveFlag for #name {
            //    const ACTIVE: &'static str = #bool_flag;
            //    fn get_active(&self) -> bool {
            //        return self.#bool_flag_ident_res;
            //    }
            //}
        };
        traits.append_all(boolflag_trait);
    }
    traits.into()
}

