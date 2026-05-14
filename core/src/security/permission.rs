//! Permission model — plugin capability restrictions

/// Permissions a plugin can request
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    StorageRead,
    StorageWrite,
    NetworkOut,
    NetworkIn,
    DhtAccess,
    PeerInfoRead,
}

pub struct PermissionManager {}

impl PermissionManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn grant(&mut self, _plugin_id: &str, _perm: Permission) {
        todo!()
    }

    pub fn revoke(&mut self, _plugin_id: &str, _perm: &Permission) {
        todo!()
    }

    pub fn check(&self, _plugin_id: &str, _perm: &Permission) -> bool {
        // Phase 1: permissive
        true
    }
}
