use app::Signal;

use super::{WlCallback, WlDisplay, WlRegistry, read_string, read_u32};
use crate::{Proxy, RawWaylandEvent, Wayland, helper};

// ── wl_display events ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum WlDisplayEvent {
    Error {
        sender: Proxy<WlDisplay>,
        object_id: crate::ObjectId,
        code: u32,
        message: String,
    },
    DeleteId {
        sender: Proxy<WlDisplay>,
        id: u32,
    },
}
impl Signal for WlDisplayEvent {}

impl WlDisplayEvent {
    pub fn parse(event: &RawWaylandEvent, wayland: &Wayland) -> Option<Self> {
        let sender = wayland.get_handle::<WlDisplay>(event.object_id)?;
        let data = &event.data;
        let mut o = 0;
        match event.opcode {
            0 => Some(WlDisplayEvent::Error {
                sender: sender.clone(),
                object_id: crate::ObjectId(read_u32(data, &mut o)?),
                code: read_u32(data, &mut o)?,
                message: read_string(data, &mut o)?,
            }),
            1 => Some(WlDisplayEvent::DeleteId {
                sender,
                id: read_u32(data, &mut o)?,
            }),
            _ => None,
        }
    }
}

pub enum WlDisplayError {
    InvalidObject,
    InvalidMethod,
    NoMemory,
    Implementation,
}

// ── wl_callback events ────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum WlCallbackEvent {
    Done {
        sender: Proxy<WlCallback>,
        callback_data: u32,
    },
}
impl Signal for WlCallbackEvent {}

impl WlCallbackEvent {
    pub fn parse(event: &RawWaylandEvent, wayland: &Wayland) -> Option<Self> {
        let sender = wayland.get_handle::<WlCallback>(event.object_id)?;
        let data = &event.data;
        let mut o = 0;
        match event.opcode {
            0 => Some(WlCallbackEvent::Done {
                sender,
                callback_data: read_u32(data, &mut o)?,
            }),
            _ => None,
        }
    }
}

// ── wl_registry events ────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum WlRegistryEvent {
    Global {
        sender: Proxy<WlRegistry>,
        name: u32,
        interface: String,
        version: u32,
    },
    GlobalDelete {
        sender: Proxy<WlRegistry>,
        name: u32,
    },
}
impl Signal for WlRegistryEvent {}

impl WlRegistryEvent {
    pub fn parse(event: &RawWaylandEvent, wayland: &Wayland) -> Option<Self> {
        let sender = wayland.get_handle::<WlRegistry>(event.object_id)?;
        let data = &event.data;
        let mut o = 0;
        match event.opcode {
            0 => Some(WlRegistryEvent::Global {
                sender: sender.clone(),
                name: read_u32(data, &mut o)?,
                interface: read_string(data, &mut o)?,
                version: read_u32(data, &mut o)?,
            }),
            1 => Some(WlRegistryEvent::GlobalDelete {
                sender,
                name: read_u32(data, &mut o)?,
            }),
            _ => None,
        }
    }
}

// ── requests ──────────────────────────────────────────────────────────────────

impl Proxy<WlDisplay> {
    pub fn sync(&self) -> Proxy<WlCallback> {
        let cb: Proxy<WlCallback> = self.conn.alloc_handle();
        let sender_id = self.object_id().expect("dead handle").0;
        let cb_id = cb.object_id().expect("just allocated").0;
        self.conn.write_raw(sender_id, 0, &cb_id.to_ne_bytes(), &[]);
        cb
    }

    pub fn get_registry(&self) -> Proxy<WlRegistry> {
        let reg: Proxy<WlRegistry> = self.conn.alloc_handle();
        let sender_id = self.object_id().expect("dead handle").0;
        let reg_id = reg.object_id().expect("just allocated").0;
        self.conn
            .write_raw(sender_id, 1, &reg_id.to_ne_bytes(), &[]);
        reg
    }
}

impl Proxy<WlRegistry> {
    pub fn bind<T: crate::Interface>(&self, name: u32, version: u32) -> Proxy<T> {
        let new_obj: Proxy<T> = self.conn.alloc_handle();
        let sender_id = self.object_id().expect("dead handle").0;
        let new_id = new_obj.object_id().expect("just allocated").0;

        let mut body = Vec::new();
        body.extend_from_slice(&name.to_ne_bytes());
        helper::encode_string(&mut body, T::NAME);
        body.extend_from_slice(&version.to_ne_bytes());
        body.extend_from_slice(&new_id.to_ne_bytes());

        self.conn.write_raw(sender_id, 0, &body, &[]);
        new_obj
    }
}
