use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use portal_core::PORTAL_PATH;
use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};

use super::interface::{
    Changed, Delete, DeletePermission, GetPermission, List, Lookup, PERMISSION_STORE_IFACE,
    PERMISSION_STORE_VERSION, PERMISSION_STORE_PATH, Set, SetPermission, SetValue,
};

#[derive(State)]
pub struct PermissionStoreBackend {
    proxy: DbusProxy<SessionBus>,
    // table -> (resource_id -> (permissions_map, extra_data))
    store: HashMap<String, HashMap<String, (HashMap<String, Vec<String>>, OwnedValue)>>,
}

impl PermissionStoreBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            store: HashMap::new(),
        }
    }
}

pub fn permission_store_backend_module<S>() -> impl RegisteredModule<PermissionStoreBackend, S> {
    Module::<PermissionStoreBackend, _, _>::new().on(
        |s: &mut PermissionStoreBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
            // Lookup
            if let Some(Ok(call)) = IncomingCall::<Lookup>::try_from(&ev.msg) {
                let (table, id) = &call.args;
                println!("[permission-store] Lookup table={table}, id={id}");
                // TODO: Retrieve permissions and data from the store
                if let Some(table_map) = s.store.get(table) {
                    if let Some((permissions, data)) = table_map.get(id) {
                        call.respond(&s.proxy, &(permissions.clone(), data.clone()));
                        return None;
                    }
                }
                let empty_permissions: HashMap<String, Vec<String>> = HashMap::new();
                let default_data = OwnedValue::try_from(Value::U8(0)).unwrap();
                call.respond(&s.proxy, &(empty_permissions, default_data));
                return None;
            }

            // Set
            if let Some(Ok(call)) = IncomingCall::<Set>::try_from(&ev.msg) {
                let (table, create, id, app_permissions, data) = &call.args;
                println!("[permission-store] Set table={table}, create={create}, id={id}");
                // TODO: Write permissions and data to the store
                let table_exists = s.store.contains_key(table);
                if !table_exists && !*create {
                    eprintln!("[permission-store] Set: Table '{table}' does not exist and create is false");
                    call.respond(&s.proxy, &());
                    return None;
                }
                let table_map = s.store.entry(table.clone()).or_default();
                table_map.insert(id.clone(), (app_permissions.clone(), data.clone()));

                // Emit Changed signal
                s.proxy.emit::<Changed>(
                    PERMISSION_STORE_PATH,
                    &(table.clone(), id.clone(), false, data.clone(), app_permissions.clone()),
                );

                call.respond(&s.proxy, &());
                return None;
            }

            // Delete
            if let Some(Ok(call)) = IncomingCall::<Delete>::try_from(&ev.msg) {
                let (table, id) = &call.args;
                println!("[permission-store] Delete table={table}, id={id}");
                // TODO: Remove the resource entry from the store
                if let Some(table_map) = s.store.get_mut(table) {
                    if let Some((permissions, data)) = table_map.remove(id) {
                        // Emit Changed signal for deletion
                        s.proxy.emit::<Changed>(
                            PERMISSION_STORE_PATH,
                            &(table.clone(), id.clone(), true, data, permissions),
                        );
                    }
                }
                call.respond(&s.proxy, &());
                return None;
            }

            // SetValue
            if let Some(Ok(call)) = IncomingCall::<SetValue>::try_from(&ev.msg) {
                let (table, create, id, data) = &call.args;
                println!("[permission-store] SetValue table={table}, create={create}, id={id}");
                // TODO: Update the data associated with the resource
                let table_exists = s.store.contains_key(table);
                if !table_exists && !*create {
                    eprintln!("[permission-store] SetValue: Table '{table}' does not exist and create is false");
                    call.respond(&s.proxy, &());
                    return None;
                }
                let table_map = s.store.entry(table.clone()).or_default();
                let entry = table_map.entry(id.clone()).or_insert_with(|| {
                    (HashMap::new(), OwnedValue::try_from(Value::U8(0)).unwrap())
                });
                entry.1 = data.clone();

                s.proxy.emit::<Changed>(
                    PERMISSION_STORE_PATH,
                    &(table.clone(), id.clone(), false, data.clone(), entry.0.clone()),
                );

                call.respond(&s.proxy, &());
                return None;
            }

            // SetPermission
            if let Some(Ok(call)) = IncomingCall::<SetPermission>::try_from(&ev.msg) {
                let (table, create, id, app_id, permissions) = &call.args;
                println!("[permission-store] SetPermission table={table}, create={create}, id={id}, app_id={app_id}");
                // TODO: Set permissions for the specific application
                let table_exists = s.store.contains_key(table);
                if !table_exists && !*create {
                    eprintln!("[permission-store] SetPermission: Table '{table}' does not exist and create is false");
                    call.respond(&s.proxy, &());
                    return None;
                }
                let table_map = s.store.entry(table.clone()).or_default();
                let entry = table_map.entry(id.clone()).or_insert_with(|| {
                    (HashMap::new(), OwnedValue::try_from(Value::U8(0)).unwrap())
                });
                entry.0.insert(app_id.clone(), permissions.clone());

                s.proxy.emit::<Changed>(
                    PERMISSION_STORE_PATH,
                    &(table.clone(), id.clone(), false, entry.1.clone(), entry.0.clone()),
                );

                call.respond(&s.proxy, &());
                return None;
            }

            // DeletePermission
            if let Some(Ok(call)) = IncomingCall::<DeletePermission>::try_from(&ev.msg) {
                let (table, id, app_id) = &call.args;
                println!("[permission-store] DeletePermission table={table}, id={id}, app_id={app_id}");
                // TODO: Delete permissions for the specific application
                if let Some(table_map) = s.store.get_mut(table) {
                    if let Some(entry) = table_map.get_mut(id) {
                        entry.0.remove(app_id);
                        s.proxy.emit::<Changed>(
                            PERMISSION_STORE_PATH,
                            &(table.clone(), id.clone(), false, entry.1.clone(), entry.0.clone()),
                        );
                    }
                }
                call.respond(&s.proxy, &());
                return None;
            }

            // GetPermission
            if let Some(Ok(call)) = IncomingCall::<GetPermission>::try_from(&ev.msg) {
                let (table, id, app_id) = &call.args;
                println!("[permission-store] GetPermission table={table}, id={id}, app_id={app_id}");
                // TODO: Retrieve permissions for the specific application
                if let Some(table_map) = s.store.get(table) {
                    if let Some(entry) = table_map.get(id) {
                        if let Some(permissions) = entry.0.get(app_id) {
                            call.respond(&s.proxy, &(permissions.clone(),));
                            return None;
                        }
                    }
                }
                let empty_permissions: Vec<String> = Vec::new();
                call.respond(&s.proxy, &(empty_permissions,));
                return None;
            }

            // List
            if let Some(Ok(call)) = IncomingCall::<List>::try_from(&ev.msg) {
                let (table,) = &call.args;
                println!("[permission-store] List table={table}");
                // TODO: List all resource IDs in the table
                let ids = if let Some(table_map) = s.store.get(table) {
                    table_map.keys().cloned().collect()
                } else {
                    Vec::new()
                };
                call.respond(&s.proxy, &(ids,));
                return None;
            }

            // Properties
            if fdo::route_properties(&s.proxy, &ev.msg, PERMISSION_STORE_IFACE, &["version"], |access| {
                match access {
                    fdo::PropAccess::Get("version") => {
                        fdo::PropReply::Value(variant(PERMISSION_STORE_VERSION))
                    }
                    fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                    _ => fdo::PropReply::Unknown,
                }
            }) {
                return None;
            }

            // Fallback
            if let DbusMessage::Call(m) = &ev.msg {
                if m.header().path().is_some_and(|p| {
                    let path_str = p.as_str();
                    path_str == PORTAL_PATH || path_str == PERMISSION_STORE_PATH
                })
                && m.header()
                    .interface()
                    .is_some_and(|i| i.as_str() == PERMISSION_STORE_IFACE)
                {
                    eprintln!("[permission-store] FALLBACK: unknown method on PermissionStore interface");
                    s.proxy.reply_unknown_method(m);
                }
            }

            None
        },
    )
}
