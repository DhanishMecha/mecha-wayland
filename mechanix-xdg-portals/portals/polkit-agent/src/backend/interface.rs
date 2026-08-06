use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

use dbus::{dbus_interface, dbus_method, dbus_signal};

/// The D-Bus object path where we serve the AuthenticationAgent interface.
/// polkitd calls back to us on the session bus at this path.
pub const POLKIT_AGENT_PATH: &str = "/org/mechanix/PolkitAgent";

/// Interface name for the agent (what polkitd calls on us).
pub const POLKIT_AGENT_IFACE: &str = "org.freedesktop.PolicyKit1.AuthenticationAgent";

// ---------------------------------------------------------------------------
// Interface we SERVE — polkitd calls these methods on us
// ---------------------------------------------------------------------------

dbus_interface!(pub AuthenticationAgentIface = POLKIT_AGENT_IFACE;
    method BeginAuthentication(
        action_id: String,
        message: String,
        icon_name: String,
        details: HashMap<String, String>,
        cookie: String,
        identities: Vec<(String, HashMap<String, OwnedValue>)>
    ) -> ();

    method CancelAuthentication(cookie: String) -> ();
);

// ---------------------------------------------------------------------------
// Methods we CALL — on org.freedesktop.PolicyKit1.Authority (system bus)
// ---------------------------------------------------------------------------

// Register our agent with polkitd. Must be called on startup.
// subject is a (subject_kind, subject_details) struct, typically
// ("unix-session", {"session-id": <s "...">}).
dbus_method!(pub RegisterAuthenticationAgent {
    dest: "org.freedesktop.PolicyKit1",
    path: "/org/freedesktop/PolicyKit1/Authority",
    iface: "org.freedesktop.PolicyKit1.Authority",
    member: "RegisterAuthenticationAgent",
    args: ((String, HashMap<String, OwnedValue>), String, String),
    reply: (),
});

// Unregister our agent with polkitd. Called on shutdown.
dbus_method!(pub UnregisterAuthenticationAgent {
    dest: "org.freedesktop.PolicyKit1",
    path: "/org/freedesktop/PolicyKit1/Authority",
    iface: "org.freedesktop.PolicyKit1.Authority",
    member: "UnregisterAuthenticationAgent",
    args: ((String, HashMap<String, OwnedValue>), String),
    reply: (),
});



// ---------------------------------------------------------------------------
// Signal we listen for on the system bus (Authority changed)
// ---------------------------------------------------------------------------

// Emitted by polkitd when actions or authorizations change.
dbus_signal!(pub AuthorityChanged {
    iface: "org.freedesktop.PolicyKit1.Authority",
    member: "Changed",
    args: (),
});
