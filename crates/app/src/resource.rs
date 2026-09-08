//! App-wide singletons, one per type, reached through proxies rather than
//! borrows of the [`App`](crate::App).
//!
//! A resource is either *in the map* or *lent out*: any number of [`Res`]
//! readers may hold it, or exactly one [`ResMut`] writer. A lookup that
//! can't be satisfied returns `None`, never panics, and the caller drops
//! what it holds and asks again.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// Marks a type as storable as an app-wide singleton.
///
/// A marker, like [`Widget`](crate::Widget): the runtime never calls into a
/// resource.
///
/// ```
/// # use app::Resource;
/// struct Clock(u64);
/// impl Resource for Clock {}
/// ```
pub trait Resource: 'static {}

/// One entry per resource type. `None` is the tombstone left while a
/// [`ResMut`] holds the value; it keeps the key so an `insert` can't slip a
/// second value in underneath the writer.
type Slot = Option<Rc<dyn Any>>;

/// The map behind every [`Res`] and [`ResMut`]. Shared by `Rc` so a proxy
/// can outlive any borrow of the app and still find its way back.
#[derive(Default)]
pub(crate) struct Resources {
    map: Rc<RefCell<HashMap<TypeId, Slot>>>,
}

impl Resources {
    pub fn new() -> Self {
        Self::default()
    }

    /// # Panics
    ///
    /// If `R` is already present, lent out to a [`ResMut`] included.
    pub fn insert<R: Resource>(&mut self, resource: R) {
        match self.map.borrow_mut().entry(TypeId::of::<R>()) {
            Entry::Occupied(_) => panic!(
                "resource `{}` is already present",
                std::any::type_name::<R>()
            ),
            Entry::Vacant(v) => {
                v.insert(Some(Rc::new(resource)));
            }
        }
    }

    /// `None` if `R` is absent or held by a [`ResMut`].
    pub fn get<R: Resource>(&self) -> Option<Res<R>> {
        let map = self.map.borrow();
        let shared = map.get(&TypeId::of::<R>())?.as_ref()?;
        let value = Rc::clone(shared)
            .downcast::<R>()
            .expect("resource slot keyed by its type");
        Some(Res { value })
    }

    /// `None` if `R` is absent, held by a [`ResMut`], or read by any
    /// [`Res`].
    pub fn get_mut<R: Resource>(&mut self) -> Option<ResMut<R>> {
        let mut map = self.map.borrow_mut();
        let slot = map.get_mut(&TypeId::of::<R>())?;
        let shared = slot.as_ref()?;
        // The map's own clone is the one strong reference a free resource
        // has; anything more is a live `Res`.
        if Rc::strong_count(shared) != 1 {
            return None;
        }
        let value = slot
            .take()
            .expect("checked above")
            .downcast::<R>()
            .expect("resource slot keyed by its type");
        Some(ResMut {
            map: Rc::clone(&self.map),
            value: Some(value),
        })
    }

    /// `None`, and nothing changes, if `R` is absent or lent out in any way.
    pub fn remove<R: Resource>(&mut self) -> Option<R> {
        let mut map = self.map.borrow_mut();
        let Entry::Occupied(entry) = map.entry(TypeId::of::<R>()) else {
            return None;
        };
        let shared = entry.get().as_ref()?;
        if Rc::strong_count(shared) != 1 {
            return None;
        }
        let value = entry
            .remove()
            .expect("checked above")
            .downcast::<R>()
            .expect("resource slot keyed by its type");
        Some(Rc::try_unwrap(value).unwrap_or_else(|_| unreachable!("strong count was 1")))
    }
}

/// A shared read of a resource. Any number may be alive at once; while any
/// is, [`resource_mut`](crate::App::resource_mut) returns `None`.
pub struct Res<R: Resource> {
    value: Rc<R>,
}

impl<R: Resource> Deref for Res<R> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &R {
        &self.value
    }
}

/// An exclusive write to a resource. While one is alive the resource is
/// out of the map, so every other lookup of it returns `None`. Drop puts it
/// back, unwinding included, unless the resource was removed in the
/// meantime, in which case the value is dropped with the proxy.
pub struct ResMut<R: Resource> {
    map: Rc<RefCell<HashMap<TypeId, Slot>>>,
    /// `Some` until `Drop` takes it.
    value: Option<Rc<R>>,
}

impl<R: Resource> Deref for ResMut<R> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &R {
        self.value.as_ref().expect("value is present until drop")
    }
}

impl<R: Resource> DerefMut for ResMut<R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut R {
        // Unique by construction: `get_mut` only hands out an `Rc` with a
        // strong count of 1, and nothing can clone it while it's here.
        Rc::get_mut(self.value.as_mut().expect("value is present until drop"))
            .expect("ResMut holds the only reference")
    }
}

impl<R: Resource> Drop for ResMut<R> {
    fn drop(&mut self) {
        let value = self.value.take().expect("dropped once");
        if let Some(slot) = self.map.borrow_mut().get_mut(&TypeId::of::<R>()) {
            debug_assert!(slot.is_none(), "tombstone overwritten while lent out");
            *slot = Some(value);
        }
    }
}
