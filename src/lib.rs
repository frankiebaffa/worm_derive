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
    dotenv::dotenv().ok();
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
        impl worm::core::AttachedDbType for #enum_ident {
            fn get_name(&self) -> String {
                return match self {
                    _ => format!("{:?}", self),
                };
            }
        }
        impl worm::core::DbCtx for #ident {
            fn init() -> #ident {
                use worm::core::DbContext as WormContext;
                use worm::core::DbObject as WormObject;
                use worm::core::sql::Connection as WormConnection;
                let mut c = WormConnection::open(":memory:").unwrap();
                let dbs = vec![ #(WormObject::new(#paths, #names), )*];
                let ctx = WormContext::new(c, dbs);
                return #ident { context: ctx, };
            }
            fn use_connection(&mut self) -> &mut worm::core::sql::Connection {
                return self.context.use_connection();
            }
            fn attach_temp_dbs(&mut self) {
                self.context.attach_temp_dbs();
            }
            fn attach_dbs(&mut self) {
                self.context.attach_dbs();
            }
            fn delete_db_files(&mut self) -> Result<(), String> {
                self.context.delete_db_files()
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
    #[darling(default)]
    insertable: bool,
    #[darling(default)]
    utc_now: bool,
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
    let mut insertable_params = Vec::new();
    let mut insertable_idents = Vec::new();
    let mut insertable_types = Vec::new();
    let mut insertable_columns = Vec::new();
    let mut utc_now_params = Vec::new();
    let mut utc_now_columns = Vec::new();
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
        let is_insertable = column.insertable;
        let is_utc_now = column.utc_now;
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
            foreign_keys.push((column.name.clone(), refr, ident.clone()));
            has_special_binding = true;
        }
        if !has_special_binding {
            normal_columns.push((ident.clone(), field.ty.clone(), column.name.clone()));
        }
        if is_insertable {
            if !is_utc_now {
                let ident_cl = ident.clone();
                let ident_str = ident_cl.to_string();
                insertable_params.push(format!(":{}", ident_str));
                insertable_idents.push(ident);
                insertable_types.push(field.ty);
                insertable_columns.push(column.name);
            } else {
                let ident_cl = ident.clone();
                let ident_str = ident_cl.to_string();
                utc_now_params.push(format!(":{}", ident_str));
                utc_now_columns.push(column.name);
            }
        } else if !is_insertable && is_utc_now {
            panic!("A column containing the utc_now property must be insertable");
        }
    }
    let mut traits = quote!{};
    let dbmodel_trait = quote! {
        impl worm::core::DbModel for #name {
            const DB: &'static str = #schema;
            const TABLE: &'static str = #table;
            const ALIAS: &'static str = #alias;
            fn from_row(row: &worm::core::sql::Row) -> Result<#name, worm::core::sql::Error>{
                use worm::core::ColumnValue;
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
            impl worm::core::ActiveFlag for #name {
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
            impl worm::core::PrimaryKey for #name {
                const PRIMARY_KEY: &'static str = #value;
                fn get_id(&self) -> i64 {
                    return self.#key;
                }
            }
        };
        primarykey_trait.to_tokens(&mut traits);
    }
    for foreign_key_item in foreign_keys {
        let column_name = foreign_key_item.0;
        let mut param = column_name.clone().to_lowercase();
        param.insert_str(0, ":");
        let type_ = foreign_key_item.1;
        let ident = foreign_key_item.2;
        let foreignkey_trait = quote! {
            impl worm::core::ForeignKey<#type_> for #name {
                const FOREIGN_KEY: &'static str = #column_name;
                const FOREIGN_KEY_PARAM: &'static str = #param;
                fn get_fk_value(&self) -> i64 {
                    return self.#ident;
                }
            }
        };
        foreignkey_trait.to_tokens(&mut traits);
    }
    if insertable_idents.len() > 0 || utc_now_params.len() > 0 {
        let mut column_names = String::new();
        let mut dlim = String::new();
        for column in insertable_columns {
            column_names.push_str(&format!("{}{}", dlim, column));
            dlim = String::from(", ");
        }
        for column in utc_now_columns {
            column_names.push_str(&format!("{}{}", dlim, column));
            dlim = String::from(", ");
        }
        let mut param_names = String::new();
        dlim = String::new();
        for param in insertable_params.clone() {
            param_names.push_str(&format!("{}{}", dlim, param));
            dlim = String::from(", ");
        }
        for param in utc_now_params.clone() {
            param_names.push_str(&format!("{}{}", dlim, param));
            dlim = String::from(", ");
        }
        let mut full_params = Vec::new();
        for param in insertable_params.clone() {
            full_params.push(param);
        }
        for param in utc_now_params.clone() {
            full_params.push(param);
        }
        let mut full_idents = Vec::new();
        for ident in insertable_idents.clone() {
            full_idents.push(ident);
        }
        for _ in utc_now_params.clone() {
            let now_ident = syn::Ident::from_string("__utc_now").unwrap();
            full_idents.push(now_ident);
        }
        let insert_function = quote! {
            impl #name {
                pub fn insert_new(db: &mut impl worm::core::DbCtx, #(#insertable_idents: #insertable_types, )*) -> Result<Self, worm::core::sql::Error> {
                    use worm::core::PrimaryKeyModel;
                    use worm::core::DbCtx;
                    let sql = format!(
                        "insert into {}.{} ( {} ) values ( {} );",
                        #schema, #table, #column_names, #param_names
                    );
                    let id;
                    {
                        let c = db.use_connection();
                        {
                            let mut tx = c.transaction()?;
                            {
                                let sp = tx.savepoint()?;
                                let __utc_now = chrono::Utc::now();
                                let params = worm::core::sql::named_params!{#(#full_params: #full_idents, )*};
                                sp.execute(&sql, params)?;
                                id = sp.last_insert_rowid();
                                sp.commit()?;
                            }
                            tx.commit()?;
                        }
                    }
                    return Self::get_by_id(db, id);
                }
            }
        };
        insert_function.to_tokens(&mut traits);
    }
    // due to use of insert_new, must go after insert_function
    if unique_name.is_some() {
        let uname_res = unique_name.unwrap();
        let value = uname_res.0;
        let key = uname_res.1;
        let uniquename_trait = quote! {
            impl worm::core::UniqueName for #name {
                const NAME: &'static str = #value;
                fn get_name(&self) -> String {
                    return self.#key.clone();
                }
            }
            impl #name {
                pub fn get_or_new(db: &mut impl worm::core::DbCtx, #(#insertable_idents: #insertable_types, )*) -> Result<Self, worm::core::sql::Error> {
                    use worm::core::UniqueNameModel;
                    match #name::get_by_name(db, &#key) {
                        Ok(s) => return Ok(s),
                        Err(_) => {},
                    }
                    #name::insert_new(db, #(#insertable_idents, )*)
                }
            }
        };
        uniquename_trait.to_tokens(&mut traits);
    }
    let mut column_consts = Vec::new();
    columns.clone().into_iter().for_each(|column| {
        column_consts.push(format_ident!("{}", column.to_uppercase()));
    });
    let col_names = columns.clone();
    let all_columns_const = quote! {
        impl #name {
            #(pub const #column_consts: &'static str = #col_names;)*
        }
    };
    all_columns_const.to_tokens(&mut traits);
    for col in normal_columns {
        let col_ident = col.0;
        let fn_name = format_ident!("get_{}", col_ident);
        let get_all_name = format_ident!("get_all_by_{}", col_ident);
        let col_param = format!(":{}", col_ident);
        let col_type = col.1;
        let col_name = col.2;
        let standard_col_trait = quote! {
            impl #name {
                pub fn #fn_name(&self) -> #col_type {
                    return self.#col_ident.clone();
                }
                pub fn #get_all_name(db: &mut impl worm::core::DbCtx, #col_ident: #col_type) -> Result<Vec<#name>, worm::core::sql::Error> {
                    use worm::core::{DbCtx, DbModel};
                    let sql = format!(
                        "select {}.* from {}.{} as {} where {}.{} = {}",
                        #name::ALIAS, #name::DB, #name::TABLE, #name::ALIAS, #name::ALIAS, #col_name, #col_param
                    );
                    let c = db.use_connection();
                    let mut stmt = c.prepare(&sql)?;
                    let params = worm::core::sql::named_params!{ #col_param: #col_ident };
                    let res: Vec<Result<#name, worm::core::sql::Error>> = stmt.query_map(params, |row| {
                        #name::from_row(&row)
                    })?.collect();
                    let mut items = Vec::new();
                    for item in res {
                        let i = item?;
                        items.push(i);
                    }
                    return Ok(items);
                }
            }
        };
        standard_col_trait.to_tokens(&mut traits);
    }
    traits.into()
}
