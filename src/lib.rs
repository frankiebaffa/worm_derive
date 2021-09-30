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
#[derive(Default, Debug, FromMeta)]
struct DbModelColumn {
    name: String,
    #[darling(default)]
    active_flag: bool,
    #[darling(default)]
    primary_key: bool,
    #[darling(default)]
    unique_name: bool,
    #[darling(default)]
    foreign_key: String,
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
    let mut has_primary_key = false;
    let mut has_unique_name = false;
    let mut active = None;
    let mut pk = None;
    let mut unique_name = None;
    let mut foreign_keys = Vec::new();
    for field in data {
        let ident = field.ident.unwrap();
        let column = field.column;
        let var = ident.to_token_stream();
        columns.push(column.name.clone());
        idents.push(ident.clone());
        vars.push(var);
        let active_flag = column.active_flag;
        let primary_key = column.primary_key;
        let uniquename = column.unique_name;
        if active_flag && !has_active_flag {
            has_active_flag = true;
            active = Some((column.name.clone(), ident.clone()));
        } else if active_flag && has_active_flag {
            panic!("A table cannot contain more than one active flag");
        }
        if primary_key && !has_primary_key {
            has_primary_key = true;
            pk = Some((column.name.clone(), ident.clone()));
        } else if primary_key && has_primary_key {
            panic!("A table cannot contain more than one primary key");
        }
        if uniquename && !has_unique_name {
            has_unique_name = true;
            unique_name = Some((column.name.clone(), ident.clone()));
        } else if uniquename && has_unique_name {
            panic!("A table cannot contain more than one unique name");
        }
        let foreign_key = column.foreign_key.clone();
        if !foreign_key.is_empty() {
            let refr = syn::Ident::from_string(&foreign_key.clone()).unwrap();
            foreign_keys.push((column.name, refr, ident));
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
    if pk.is_some() {
        let pk_res = pk.unwrap();
        let value = pk_res.0;
        let key = pk_res.1;
        let primarykey_trait = quote! {
            impl worm::traits::primarykey::PrimaryKey for #name {
                const PRIMARY_KEY: &'static str = #value;
                fn get_id(&self) -> i64 {
                    return self.#key;
                }
            }
        };
        primarykey_trait.to_tokens(&mut traits);
    }
    if unique_name.is_some() {
        let uname_res = unique_name.unwrap();
        let value = uname_res.0;
        let key = uname_res.1;
        let uniquename_trait = quote! {
            impl worm::traits::uniquename::UniqueName for #name {
                const NAME: &'static str = #value;
                fn get_name(&self) -> String {
                    return self.#key.clone();
                }
            }
        };
        uniquename_trait.to_tokens(&mut traits);
    }
    for foreign_key_item in foreign_keys {
        let column_name = foreign_key_item.0;
        let mut param = column_name.clone().to_lowercase();
        param.insert_str(0, ":");
        let type_ = foreign_key_item.1;
        let ident = foreign_key_item.2;
        let foreignkey_trait = quote! {
            impl worm::traits::foreignkey::ForeignKey<#type_> for #name {
                const FOREIGN_KEY: &'static str = #column_name;
                const FOREIGN_KEY_PARAM: &'static str = #param;
                fn get_fk_value(&self) -> i64 {
                    return self.#ident;
                }
            }
        };
        foreignkey_trait.to_tokens(&mut traits);
    }
    traits.into()
}

