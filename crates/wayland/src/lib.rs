//! The Wayland client: the connection as a resource, every protocol object
//! as a typed [`Proxy`], every protocol event as a signal.
//!
//! `WaylandModule::install` connects to the compositor, or panics. It sends
//! `get_registry` and a `sync`, then waits on the ring until the sync is
//! answered, binding every advertised global it has an interface for into
//! [`Globals`]. When install returns, the compositor's capabilities are
//! known and any later module takes what it needs from `Globals`.
//!
//! After install nothing blocks. A system on `IoEvent` turns a read into
//! typed signals, one per message, for the systems that listen; requests
//! written through a proxy only buffer, and a system on `BeforeWait`
//! flushes them and re-arms the read, so every batch ends with at most one
//! send and one receive in flight. Proxies keep an `Rc` into the connection
//! so a request can be sent from anywhere, and a bound global can sit in
//! any resource; the ring is reached only through the app.
//!
//! This crate knows no window and no layout. Presentation is a crate above.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::mem;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use app::{App, Module, Resource};
use io_uring::{opcode, types};
use ring::{BeforeWait, IoEvent, Ring, Token};

pub(crate) mod helper;
pub mod proto;

pub use proto::*;

pub mod prelude {
    pub use crate::{Globals, Interface, ObjectId, Proxy, Wayland, WaylandModule};
}

/// A protocol interface: its name on the wire and the highest version this
/// crate speaks.
pub trait Interface {
    const NAME: &'static str;
    const VERSION: u32;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ObjectId(pub(crate) u32);

/// The wl_display singleton always uses object ID 1.
pub const DISPLAY_OBJECT_ID: ObjectId = ObjectId(1);

/// One message off the wire, before it is parsed into a typed event.
#[derive(Debug)]
pub struct RawWaylandEvent {
    pub(crate) object_id: ObjectId,
    pub(crate) opcode: u32,
    pub(crate) data: Vec<u8>,
}

const READ_BUF_SIZE: usize = 65536;
const CMSG_BUF_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

/// Everything the socket needs, boxed once and never moved: the ring holds
/// raw pointers into the buffers and message headers while an operation is
/// in flight.
pub(crate) struct Inner {
    fd: RawFd,

    read_buf: Vec<u8>,
    cmsg_recv_buf: Vec<u8>,
    recv_iov: libc::iovec,
    recv_msghdr: libc::msghdr,
    /// Bytes read but not yet a whole message.
    inbox: Vec<u8>,

    write_buf: Vec<u8>,
    write_fds_buf: Vec<OwnedFd>,
    write_in_flight: Vec<u8>,
    write_fds_in_flight: Vec<OwnedFd>,
    write_cmsg_buf: Vec<u8>,
    send_iov: libc::iovec,
    send_msghdr: libc::msghdr,

    fd_queue: VecDeque<OwnedFd>,

    read_token: Option<Token>,
    write_token: Option<Token>,
    next_id: u32,
    object_slots: HashMap<ObjectId, Rc<ObjectId>>,
    object_interfaces: HashMap<ObjectId, &'static str>,
    deleted_object_ids: HashSet<ObjectId>,
}

impl Inner {
    fn new(fd: RawFd) -> Self {
        let display_id = DISPLAY_OBJECT_ID;
        let mut object_slots = HashMap::new();
        let mut object_interfaces = HashMap::new();
        object_slots.insert(display_id, Rc::new(display_id));
        object_interfaces.insert(display_id, "wl_display");

        Self {
            fd,
            read_buf: vec![0u8; READ_BUF_SIZE],
            cmsg_recv_buf: vec![0u8; CMSG_BUF_SIZE],
            recv_iov: unsafe { mem::zeroed() },
            recv_msghdr: unsafe { mem::zeroed() },
            inbox: Vec::new(),
            write_buf: Vec::with_capacity(4096),
            write_fds_buf: Vec::new(),
            write_in_flight: Vec::new(),
            write_fds_in_flight: Vec::new(),
            write_cmsg_buf: Vec::new(),
            send_iov: unsafe { mem::zeroed() },
            send_msghdr: unsafe { mem::zeroed() },
            fd_queue: VecDeque::new(),
            read_token: None,
            write_token: None,
            next_id: 2,
            object_slots,
            object_interfaces,
            deleted_object_ids: HashSet::new(),
        }
    }

    fn arm_read(&mut self, ring: &mut Ring) {
        if self.read_token.is_some() {
            return;
        }
        // SAFETY: the buffers and headers live in this boxed `Inner`, which
        // the resource keeps alive for the life of the connection; the
        // pointers stay valid until the completion clears `read_token`.
        unsafe {
            let iov_ptr = &mut self.recv_iov as *mut libc::iovec;
            (*iov_ptr).iov_base = self.read_buf.as_mut_ptr() as *mut libc::c_void;
            (*iov_ptr).iov_len = self.read_buf.len();

            let msghdr_ptr = &mut self.recv_msghdr as *mut libc::msghdr;
            std::ptr::write(msghdr_ptr, mem::zeroed());
            (*msghdr_ptr).msg_iov = iov_ptr;
            (*msghdr_ptr).msg_iovlen = 1;
            (*msghdr_ptr).msg_control = self.cmsg_recv_buf.as_mut_ptr() as *mut libc::c_void;
            (*msghdr_ptr).msg_controllen = self.cmsg_recv_buf.len() as libc::size_t;

            let sqe = opcode::RecvMsg::new(types::Fd(self.fd), msghdr_ptr).build();
            self.read_token = Some(ring.push(sqe));
        }
    }

    fn extract_recv_fds(&mut self) {
        unsafe {
            let msghdr_ptr = &self.recv_msghdr as *const libc::msghdr;
            let mut cmsg = libc::CMSG_FIRSTHDR(msghdr_ptr);
            while !cmsg.is_null() {
                if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                    let data_len =
                        ((*cmsg).cmsg_len as usize).saturating_sub(libc::CMSG_LEN(0) as usize);
                    let n_fds = data_len / mem::size_of::<RawFd>();
                    let fd_ptr = libc::CMSG_DATA(cmsg) as *const RawFd;
                    for i in 0..n_fds {
                        self.fd_queue
                            .push_back(OwnedFd::from_raw_fd(*fd_ptr.add(i)));
                    }
                }
                cmsg = libc::CMSG_NXTHDR(msghdr_ptr, cmsg);
            }
        }
    }

    /// Submits what is buffered, if nothing is in flight. One send at a time
    /// keeps the fds and their bytes in the same message.
    fn flush(&mut self, ring: &mut Ring) {
        if self.write_token.is_some() || self.write_buf.is_empty() {
            return;
        }
        mem::swap(&mut self.write_in_flight, &mut self.write_buf);
        mem::swap(&mut self.write_fds_in_flight, &mut self.write_fds_buf);

        if self.write_fds_in_flight.is_empty() {
            let sqe = opcode::Write::new(
                types::Fd(self.fd),
                self.write_in_flight.as_ptr(),
                self.write_in_flight.len() as u32,
            )
            .build();
            self.write_token = Some(ring.push(sqe));
            return;
        }

        // SAFETY: as for `arm_read`; the fds are kept in `write_fds_in_flight`
        // until the completion.
        unsafe {
            let fd_count = self.write_fds_in_flight.len();
            let fd_bytes = (fd_count * mem::size_of::<RawFd>()) as u32;
            let cmsg_space = libc::CMSG_SPACE(fd_bytes) as usize;
            self.write_cmsg_buf.resize(cmsg_space, 0);

            let iov_ptr = &mut self.send_iov as *mut libc::iovec;
            (*iov_ptr).iov_base = self.write_in_flight.as_ptr() as *mut libc::c_void;
            (*iov_ptr).iov_len = self.write_in_flight.len();

            let msghdr_ptr = &mut self.send_msghdr as *mut libc::msghdr;
            std::ptr::write(msghdr_ptr, mem::zeroed());
            (*msghdr_ptr).msg_iov = iov_ptr;
            (*msghdr_ptr).msg_iovlen = 1;
            (*msghdr_ptr).msg_control = self.write_cmsg_buf.as_mut_ptr() as *mut libc::c_void;
            (*msghdr_ptr).msg_controllen = cmsg_space as libc::size_t;

            let cmsg = libc::CMSG_FIRSTHDR(msghdr_ptr);
            (*cmsg).cmsg_level = libc::SOL_SOCKET;
            (*cmsg).cmsg_type = libc::SCM_RIGHTS;
            (*cmsg).cmsg_len = libc::CMSG_LEN(fd_bytes) as _;

            let raw_fds: Vec<RawFd> = self
                .write_fds_in_flight
                .iter()
                .map(|f| f.as_raw_fd())
                .collect();
            std::ptr::copy_nonoverlapping(
                raw_fds.as_ptr(),
                libc::CMSG_DATA(cmsg) as *mut RawFd,
                fd_count,
            );

            let sqe =
                opcode::SendMsg::new(types::Fd(self.fd), msghdr_ptr as *const libc::msghdr).build();
            self.write_token = Some(ring.push(sqe));
        }
    }

    fn alloc_id(&mut self) -> ObjectId {
        if let Some(&id) = self.deleted_object_ids.iter().next() {
            self.deleted_object_ids.remove(&id);
            id
        } else {
            let id = ObjectId(self.next_id);
            self.next_id += 1;
            id
        }
    }

    fn invalidate_object(&mut self, id: ObjectId) {
        self.object_slots.remove(&id);
        self.object_interfaces.remove(&id);
        self.deleted_object_ids.insert(id);
    }
}

/// What a completion on the socket turned out to be.
enum Completion {
    Read(Vec<RawWaylandEvent>),
    Write,
    NotOurs,
}

/// The shared connection: what a [`Proxy`] holds so a request can be
/// written from anywhere, and what the [`Wayland`] resource wraps.
#[derive(Clone)]
pub struct Connection(Rc<RefCell<Inner>>);

impl Connection {
    pub fn new_handle<T: Interface>(&self, id: ObjectId) -> Proxy<T> {
        let mut inner = self.0.borrow_mut();
        let rc = Rc::new(id);
        inner.object_slots.insert(id, rc.clone());
        inner.object_interfaces.insert(id, T::NAME);
        Proxy {
            slot: Rc::downgrade(&rc),
            conn: self.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn get_handle<T: Interface>(&self, id: ObjectId) -> Option<Proxy<T>> {
        let inner = self.0.borrow();
        inner.object_slots.get(&id).map(|rc| Proxy {
            slot: Rc::downgrade(rc),
            conn: self.clone(),
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn alloc_handle<T: Interface>(&self) -> Proxy<T> {
        let id = self.0.borrow_mut().alloc_id();
        self.new_handle(id)
    }

    pub fn get_interface(&self, id: ObjectId) -> Option<&'static str> {
        self.0.borrow().object_interfaces.get(&id).copied()
    }

    /// The next fd the compositor sent, in the order they arrived.
    pub fn take_fd(&self) -> Option<OwnedFd> {
        self.0.borrow_mut().fd_queue.pop_front()
    }

    /// Sends what is buffered and makes sure a read is outstanding. On the
    /// connection rather than the resource so a caller can clone it out of
    /// the app and then borrow the ring from the same app.
    pub fn flush(&self, ring: &mut Ring) {
        let mut inner = self.0.borrow_mut();
        inner.flush(ring);
        inner.arm_read(ring);
    }

    /// Buffers one request. Nothing is submitted here; the flush on `Tick`
    /// sends it.
    pub(crate) fn write_raw(
        &self,
        sender_id: u32,
        opcode: u16,
        body: &[u8],
        fds: &[BorrowedFd<'_>],
    ) {
        let mut inner = self.0.borrow_mut();
        let total = (8 + body.len()) as u32;
        inner.write_buf.extend_from_slice(&sender_id.to_ne_bytes());
        inner
            .write_buf
            .extend_from_slice(&((total << 16) | opcode as u32).to_ne_bytes());
        inner.write_buf.extend_from_slice(body);
        for fd in fds {
            inner
                .write_fds_buf
                .push(fd.try_clone_to_owned().expect("failed to dup fd"));
        }
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// The connection to the compositor, as a resource.
pub struct Wayland {
    conn: Connection,
    /// The sync sent at install; answered means connected.
    handshake: Option<Proxy<WlCallback>>,
}
impl Resource for Wayland {}

impl Wayland {
    /// Connects to `$WAYLAND_DISPLAY` under `$XDG_RUNTIME_DIR`, or panics.
    pub fn connect() -> Self {
        let runtime_dir = env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is not set");
        let display = env::var("WAYLAND_DISPLAY").expect("WAYLAND_DISPLAY is not set");
        let path = PathBuf::from(runtime_dir).join(&display);
        let stream = UnixStream::connect(&path).unwrap_or_else(|e| {
            panic!(
                "cannot connect to the compositor at {}: {e}",
                path.display()
            )
        });
        stream.set_nonblocking(true).expect("set_nonblocking");
        let fd = stream.into_raw_fd();
        Self {
            conn: Connection(Rc::new(RefCell::new(Inner::new(fd)))),
            handshake: None,
        }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn display(&self) -> Proxy<WlDisplay> {
        self.conn
            .get_handle::<WlDisplay>(DISPLAY_OBJECT_ID)
            .expect("display always exists")
    }

    pub fn new_handle<T: Interface>(&self, id: ObjectId) -> Proxy<T> {
        self.conn.new_handle(id)
    }

    pub fn get_handle<T: Interface>(&self, id: ObjectId) -> Option<Proxy<T>> {
        self.conn.get_handle(id)
    }

    pub fn get_interface(&self, id: ObjectId) -> Option<&'static str> {
        self.conn.get_interface(id)
    }

    pub fn take_fd(&self) -> Option<OwnedFd> {
        self.conn.take_fd()
    }

    pub fn invalidate_object(&self, id: ObjectId) {
        self.conn.0.borrow_mut().invalidate_object(id);
    }

    fn complete(&self, token: Token, result: i32) -> Completion {
        let mut inner = self.conn.0.borrow_mut();
        if Some(token) == inner.read_token {
            inner.read_token = None;
            if result <= 0 {
                panic!("the compositor closed the connection (recvmsg returned {result})");
            }
            let n = result as usize;
            let mut chunk = mem::take(&mut inner.inbox);
            chunk.extend_from_slice(&inner.read_buf[..n]);
            inner.extract_recv_fds();
            let (messages, rest) = helper::parse_messages(chunk);
            inner.inbox = rest;
            Completion::Read(messages)
        } else if Some(token) == inner.write_token {
            inner.write_token = None;
            if result < 0 {
                panic!("write to the compositor failed ({result})");
            }
            inner.write_in_flight.clear();
            inner.write_fds_in_flight.clear();
            Completion::Write
        } else {
            Completion::NotOurs
        }
    }
}

// ---------------------------------------------------------------------------
// Proxy
// ---------------------------------------------------------------------------

/// A typed reference to a protocol object, libwayland's word for the
/// client's side of one. Not a `Handle`: that names a node. Dead once the
/// compositor has deleted the object; `object_id` then returns `None`.
pub struct Proxy<T: Interface> {
    slot: Weak<ObjectId>,
    pub(crate) conn: Connection,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Interface> Proxy<T> {
    pub fn object_id(&self) -> Option<ObjectId> {
        self.slot.upgrade().map(|rc| *rc)
    }

    pub fn is_alive(&self) -> bool {
        self.slot.strong_count() > 0
    }
}

impl<T: Interface> PartialEq for Proxy<T> {
    fn eq(&self, other: &Self) -> bool {
        self.object_id() == other.object_id()
    }
}

impl<T: Interface> Eq for Proxy<T> {}

impl<T: Interface> std::hash::Hash for Proxy<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.object_id().hash(state)
    }
}

impl<T: Interface> Clone for Proxy<T> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
            conn: self.conn.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Interface> std::fmt::Debug for Proxy<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proxy")
            .field("interface", &T::NAME)
            .field("object_id", &self.slot.upgrade().map(|rc| rc.0))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------

/// What the compositor advertised at connect, bound: one handle per
/// interface this crate knows. A snapshot; globals advertised later arrive
/// as `WlRegistryEvent::Global` and are not here.
#[derive(Default)]
pub struct Globals {
    bound: HashMap<TypeId, Box<dyn Any>>,
    advertised: Vec<(u32, String, u32)>,
}
impl Resource for Globals {}

impl Globals {
    pub fn get<T: Interface + 'static>(&self) -> Option<&Proxy<T>> {
        self.bound
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<Proxy<T>>())
    }

    /// The bound handle for `T`, or a panic naming the interface: a module
    /// that needs a protocol the compositor lacks cannot install.
    pub fn require<T: Interface + 'static>(&self) -> &Proxy<T> {
        self.get::<T>()
            .unwrap_or_else(|| panic!("the compositor does not advertise {}", T::NAME))
    }

    pub fn insert<T: Interface + 'static>(&mut self, h: Proxy<T>) {
        self.bound.insert(TypeId::of::<T>(), Box::new(h));
    }

    /// Every global the registry announced, bound or not: name, interface,
    /// version.
    pub fn advertised(&self) -> &[(u32, String, u32)] {
        &self.advertised
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

/// Connects, binds every known global, or panics. Needs the [`Ring`]
/// resource, so `RingModule` installs first.
pub struct WaylandModule;

impl Module for WaylandModule {
    fn install(self, app: &mut App) {
        app.insert_resource(Wayland::connect())
            .insert_resource(Globals::default())
            .system(on_io)
            .system(on_before_wait)
            .system(on_registry)
            .system(on_display)
            .system(on_callback);

        // The handshake: registry, sync, and the one blocking wait outside
        // the runner. Every completion seen here is ours: nothing else has
        // submitted yet.
        {
            let mut wayland = app.resource_mut::<Wayland>().expect("just inserted");
            let display = wayland.display();
            display.get_registry();
            wayland.handshake = Some(display.sync());
        }
        while app
            .resource::<Wayland>()
            .expect("no Wayland writer is alive during the handshake")
            .handshake
            .is_some()
        {
            flush(app);
            let completions = app
                .resource_mut::<Ring>()
                .expect("WaylandModule needs the Ring: install RingModule first")
                .wait();
            for c in completions {
                app.signal(c);
            }
            app.flush();
        }
    }
}

/// Sends what is buffered and re-arms the read. The connection is cloned
/// out so the ring can be borrowed from the same app.
fn flush(app: &mut App) {
    let conn = app
        .resource::<Wayland>()
        .expect("no Wayland writer is alive at a flush")
        .connection()
        .clone();
    let mut ring = app
        .resource_mut::<Ring>()
        .expect("no Ring holder is alive at a flush");
    conn.flush(&mut ring);
}

fn on_before_wait(app: &mut App, _: &BeforeWait) {
    flush(app);
}

/// A completion on the socket. A read becomes typed signals, one per
/// message; the read is re-armed at the next flush.
fn on_io(app: &mut App, ev: &IoEvent) {
    let completion = app
        .resource::<Wayland>()
        .expect("no Wayland writer is alive across a completion")
        .complete(ev.token, ev.result);
    if let Completion::Read(messages) = completion {
        for raw in &messages {
            proto::dispatch(app, raw);
        }
    }
}

/// Every global the registry announces is recorded, and bound if this crate
/// generated its interface.
fn on_registry(app: &mut App, ev: &WlRegistryEvent) {
    if let WlRegistryEvent::Global {
        sender,
        name,
        interface,
        version,
    } = ev
    {
        app.resource_mut::<Globals>()
            .expect("no Globals holder is alive across a registry event")
            .advertised
            .push((*name, interface.clone(), *version));
        proto::bind_global(app, sender, *name, interface, *version);
    }
}

fn on_display(app: &mut App, ev: &WlDisplayEvent) {
    match ev {
        WlDisplayEvent::Error {
            object_id,
            code,
            message,
            ..
        } => panic!(
            "wayland protocol error on object {}: code {code}: {message}",
            object_id.0
        ),
        WlDisplayEvent::DeleteId { id, .. } => {
            if let Some(wayland) = app.resource::<Wayland>() {
                wayland.invalidate_object(ObjectId(*id));
            }
        }
    }
}

/// The handshake's sync is answered: connected.
fn on_callback(app: &mut App, ev: &WlCallbackEvent) {
    let WlCallbackEvent::Done { sender, .. } = ev;
    let Some(mut wayland) = app.resource_mut::<Wayland>() else {
        return;
    };
    if wayland.handshake.as_ref() == Some(sender) {
        wayland.handshake = None;
    }
}
