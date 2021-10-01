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
        format_ident,
        quote
    },
    syn::{
        DeriveInput,
        parse_macro_input,

    },
};
#[derive(Default, FromMeta)]
struct WormDbVar {
    name: String,
}
#[derive(FromDeriveInput)]
#[darling(attributes(db))]
struct WormDbOpts {
    ident: syn::Ident,
    var: WormDbVar,
}
#[proc_macro_derive(WormDb, attributes(db))]
pub fn derive_wormdb(input: TokenStream) -> TokenStream {
    let d_input = parse_macro_input!(input as DeriveInput);
    let wormdb = WormDbOpts::from_derive_input(&d_input).unwrap();
    match dotenv::dotenv() {
        Ok(_) => {},
        Err(_) => {},
    }
    let dbs = match std::env::var(&wormdb.var.name) {
        Ok(dbs) => dbs,
        Err(_) => panic!("Failed to construct wormdb, environment variable {} not found", &wormdb.var.name),
    };
    let db_split = dbs.split(",");
    let mut names = Vec::new();
    let mut paths = Vec::new();
    for mut db_data in db_split {
        db_data = db_data.trim();
        let mut name_path = db_data.split("@");
        let name = name_path.nth(0).expect("Failed to get name of wormdb, environment variable value is in improper format");
        let path = name_path.nth(0).expect("Failed to get path of wormdb, environment variable value is in improper format");
        names.push(name.trim().to_string());
        paths.push(path.trim().to_string())
    }
    let ident = wormdb.ident;
    let mut db_name_idents = Vec::new();
    for name in names.clone() {
        db_name_idents.push(format_ident!("{}", name));
    }
    let enum_ident = format_ident!("AttachedTo{}", ident);
    let implementation = quote! {
        #[derive(Debug)]
        pub enum #enum_ident {
            #(#db_name_idents,)*
        }
        impl worm::traits::dbmodel::AttachedDbType for #enum_ident {
            fn get_name(&self) -> String {
                return match self {
                    _ => format!("{:?}", self),
                };
            }
        }
        impl worm::traits::dbctx::DbCtx for #ident {
            fn init() -> #ident {
                use worm::structs::database::DbContext as WormContext;
                use worm::structs::database::DbObject as WormObject;
                use rusqlite::Connection as WormConnection;
                let mut c = WormConnection::open(":memory:").unwrap();
                let dbs = vec![ #(WormObject::new(#paths, #names), )*];
                let ctx = WormContext::new(c, dbs);
                return #ident { context: ctx, };
            }
            fn use_connection(&mut self) -> &mut rusqlite::Connection {
                return self.context.use_connection();
            }
        }
    };
    implementation.into()
}
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
    ty: syn::Type,
}
#[derive(Default, FromMeta)]
struct DbModelTable {
    db: String,
    schema: String,
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
    let db_ident = format_ident!("{}", db);
    let schema = opts.table.schema.as_str();
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
    let mut normal_columns = Vec::new();
    for field in data {
        let mut has_special_binding = false;
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
            has_special_binding = true;
        } else if active_flag && has_active_flag {
            panic!("A table cannot contain more than one active flag");
        }
        if primary_key && !has_primary_key {
            has_primary_key = true;
            pk = Some((column.name.clone(), ident.clone()));
            has_special_binding = true;
        } else if primary_key && has_primary_key {
            panic!("A table cannot contain more than one primary key");
        }
        if uniquename && !has_unique_name {
            has_unique_name = true;
            unique_name = Some((column.name.clone(), ident.clone()));
            has_special_binding = true;
        } else if uniquename && has_unique_name {
            panic!("A table cannot contain more than one unique name");
        }
        let foreign_key = column.foreign_key.clone();
        if !foreign_key.is_empty() {
            let refr = syn::Ident::from_string(&foreign_key.clone()).unwrap();
            foreign_keys.push((column.name, refr, ident.clone()));
            has_special_binding = true;
        }
        if !has_special_binding {
            normal_columns.push((ident, field.ty));
        }
    }
    let mut traits = quote!{};
    for col in normal_columns {
        let col_ident = col.0;
        let fn_name = format_ident!("get_{}", col_ident);
        let col_type = col.1;
        let standard_col_trait = quote! {
            impl #name {
                pub fn #fn_name(&self) -> #col_type {
                    return self.#col_ident.clone();
                }
            }
        };
        standard_col_trait.to_tokens(&mut traits);
    }
    let attached_db_type = format_ident!("{}", schema);
    let attached_enum = format_ident!("AttachedTo{}", db);
    let dbmodel_trait = quote! {
        impl worm::traits::dbmodel::DbModel<#db_ident, #attached_enum> for #name {
            const DB: &'static str = #schema;
            const TABLE: &'static str = #table;
            const ALIAS: &'static str = #alias;
            fn from_row(row: &rusqlite::Row) -> Result<#name, rusqlite::Error>{
                use worm::traits::helpers::ColumnValue;
                #(let #idents = row.value(&#columns)?;)*
                return Ok(#name { #(#idents: #idents, )*});
            }
            fn get_attached_db_type() -> #attached_enum {
                return #attached_enum::#attached_db_type;
            }
        }
    };
    dbmodel_trait.to_tokens(&mut traits);
    if active.is_some() {
        let active_res = active.unwrap();
        let value = active_res.0;
        let key = active_res.1;
        let activeflag_trait = quote! {
            impl worm::traits::activeflag::ActiveFlag<#db_ident, #attached_enum> for #name {
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
            impl worm::traits::primarykey::PrimaryKey<#db_ident, #attached_enum> for #name {
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
            impl worm::traits::uniquename::UniqueName<#db_ident, #attached_enum> for #name {
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
            impl worm::traits::foreignkey::ForeignKey<#db_ident, #attached_enum, #type_> for #name {
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

